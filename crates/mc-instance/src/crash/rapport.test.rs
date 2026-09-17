use super::{find, now};
use crate::essais::Arbre;

const TRACE: &str = "\
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package com.mojang.blaze3d.systems
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source)";

/// Recule la date de dernière écriture d'un fichier, pour le faire passer
/// pour un reste d'une partie précédente.
fn vieillir(chemin: &std::path::Path, secondes: u64) {
    let quand = std::time::SystemTime::now() - std::time::Duration::from_secs(secondes);
    let fichier = std::fs::OpenOptions::new()
        .write(true)
        .open(chemin)
        .unwrap();
    fichier
        .set_times(std::fs::FileTimes::new().set_modified(quand))
        .unwrap();
}

/// L'instant du lancement, reculé d'une seconde.
///
/// La date d'écriture d'un fichier est moins fine que l'horloge : un fichier
/// créé juste après `now()` peut porter une date arrondie en deçà, et passer
/// pour un reste de la partie précédente. Le jeu, lui, tourne des minutes entre
/// les deux — la marge n'existe que pour ce test.
fn lancement() -> std::time::SystemTime {
    now() - std::time::Duration::from_secs(1)
}

fn ecrire(arbre: &Arbre, relatif: &str, contenu: &str) -> std::path::PathBuf {
    let chemin = arbre.game_dir().join(relatif);
    std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    std::fs::write(&chemin, contenu).unwrap();
    chemin
}

/// Le rapport de crash est le plus riche : description, trace, mods chargés,
/// pilote graphique. C'est lui qu'on préfère quand il existe.
#[test]
fn le_rapport_de_crash_est_prefere_au_journal() {
    let arbre = Arbre::neuf("rapport-prefere");
    let debut = lancement();
    ecrire(
        &arbre,
        "logs/latest.log",
        "java.io.IOException: autre chose",
    );
    let rapport = ecrire(&arbre, "crash-reports/crash-2026-09-17.txt", TRACE);

    let trouve = find(&arbre.game_dir(), debut).expect("un plantage est trouvé");

    assert_eq!(trouve.exception, "java.lang.module.ResolutionException");
    assert_eq!(trouve.source, rapport);
}

/// Une erreur de chargement de mods se trouve dans le journal alors qu'aucun
/// rapport n'est produit : la JVM s'arrête avant que le jeu n'existe. C'est le
/// cas le plus fréquent avec un modpack.
#[test]
fn a_defaut_de_rapport_le_journal_est_lu() {
    let arbre = Arbre::neuf("rapport-journal");
    let debut = lancement();
    let journal = ecrire(&arbre, "logs/latest.log", TRACE);

    let trouve = find(&arbre.game_dir(), debut).expect("le journal suffit");

    assert_eq!(trouve.exception, "java.lang.module.ResolutionException");
    assert_eq!(trouve.source, journal);
}

/// Un rapport de trois jours attribué au lancement du jour enverrait sur une
/// fausse piste.
#[test]
fn un_rapport_anterieur_au_lancement_est_ignore() {
    let arbre = Arbre::neuf("rapport-vieux");
    let chemin = ecrire(&arbre, "crash-reports/crash-vieux.txt", TRACE);
    vieillir(&chemin, 3 * 24 * 3600);
    let journal = ecrire(&arbre, "logs/latest.log", TRACE);
    vieillir(&journal, 3 * 24 * 3600);

    assert!(find(&arbre.game_dir(), now()).is_none());
}

/// Entre deux rapports de cette exécution, c'est le dernier écrit qui décrit
/// l'arrêt.
#[test]
fn le_rapport_le_plus_recent_l_emporte() {
    let arbre = Arbre::neuf("rapport-recent");
    let debut = lancement();
    let ancien = ecrire(
        &arbre,
        "crash-reports/crash-a.txt",
        "java.io.IOException: le premier",
    );
    vieillir(&ancien, 1);
    let recent = ecrire(&arbre, "crash-reports/crash-b.txt", TRACE);

    let trouve = find(&arbre.game_dir(), debut).unwrap();
    assert_eq!(trouve.source, recent);
}

/// Une partie qui se termine normalement laisse un journal sans exception :
/// n'y rien trouver est le cas nominal, pas un échec.
#[test]
fn une_partie_sans_incident_ne_donne_rien() {
    let arbre = Arbre::neuf("rapport-propre");
    let debut = lancement();
    ecrire(&arbre, "logs/latest.log", "[INFO]: Stopping worker threads");

    assert!(find(&arbre.game_dir(), debut).is_none());
}

#[test]
fn un_repertoire_de_jeu_vide_ne_produit_rien() {
    let arbre = Arbre::neuf("rapport-vide");
    assert!(find(&arbre.game_dir(), now()).is_none());
}

/// Les fichiers du répertoire qui ne sont pas des rapports — une note, une
/// capture — ne doivent pas être pris pour tels.
#[test]
fn seuls_les_fichiers_nommes_crash_sont_examines() {
    let arbre = Arbre::neuf("rapport-etranger");
    let debut = lancement();
    ecrire(&arbre, "crash-reports/notes.txt", TRACE);

    assert!(find(&arbre.game_dir(), debut).is_none());
}
