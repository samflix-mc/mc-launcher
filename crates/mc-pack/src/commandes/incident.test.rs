use super::{report_game_crash, report_game_error};
use crate::commandes::essais::{Atelier, entree, verrou};

const TRACE: &str = "\
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source)";

/// Le lancement, reculé d'une seconde : la date d'écriture d'un fichier est
/// moins fine que l'horloge, et le jeu tourne des minutes entre les deux.
fn lancement() -> std::time::SystemTime {
    std::time::SystemTime::now() - std::time::Duration::from_secs(1)
}

/// Le launcher et le jeu sont deux processus : la trace Java reste du côté du
/// jeu, et c'est le seul moment où elle est disponible. Aucun client Sentry
/// n'est initialisé ici — ce qui est vérifié est la collecte, pas l'envoi.
#[test]
fn un_plantage_du_jeu_est_repris_dans_les_journaux_de_la_partie() {
    let atelier = Atelier::neuf("incident-crash");
    atelier.pack_installe(vec![entree("jei", "both")]);
    let instance = atelier.options().layout.instance("samflix");

    let debut = lancement();
    let logs = instance.game_dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("latest.log"), TRACE).unwrap();

    report_game_crash(
        &instance,
        &verrou(vec![entree("jei", "both")]),
        "neoforge-21.1.250",
        debut,
        1,
    );
}

/// Aucune trace exploitable : l'incident du launcher reste, et il dit au moins
/// où chercher. Ce chemin ne doit pas paniquer — c'est celui d'un joueur dont
/// le jeu a disparu sans rien écrire.
#[test]
fn un_plantage_sans_trace_ne_fait_pas_paniquer_le_launcher() {
    let atelier = Atelier::neuf("incident-sans-trace");
    atelier.pack_installe(Vec::new());
    let instance = atelier.options().layout.instance("samflix");

    report_game_crash(
        &instance,
        &verrou(Vec::new()),
        "neoforge-21.1.250",
        lancement(),
        1,
    );
}

/// Le contexte joint est celui qu'on demanderait sinon au joueur : versions,
/// mods présents, et d'où vient la trace.
#[test]
fn une_exception_traversee_se_distingue_d_une_erreur_fatale() {
    let atelier = Atelier::neuf("incident-contexte");
    atelier.pack_installe(vec![entree("jei", "both")]);
    let instance = atelier.options().layout.instance("samflix");

    let crash = mc_instance::crash::parse(TRACE).expect("une exception");
    let lock = verrou(vec![entree("jei", "both")]);

    // Sans client Sentry, l'identifiant rendu est nul — ce qui compte ici est
    // que les deux chemins, fatal et traversé, s'assemblent sans paniquer.
    let fatale = report_game_error(&instance, &lock, "neoforge-21.1.250", &crash, Some(1));
    let traversee = report_game_error(&instance, &lock, "neoforge-21.1.250", &crash, None);
    assert_eq!(fatale, traversee, "aucun client : les deux sont nuls");
}
