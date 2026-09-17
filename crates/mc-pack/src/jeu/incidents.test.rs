use super::{report_game_crash, report_game_error};
use crate::essais::{Atelier, entree, verrou};

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
    atelier.pack_installe(vec![entree("jei", "both", None)]);
    let instance = atelier.options().layout.instance("samflix");

    let debut = lancement();
    let logs = instance.game_dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("latest.log"), TRACE).unwrap();

    report_game_crash(
        &instance,
        &verrou(vec![entree("jei", "both", None)]),
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
    atelier.pack_installe(vec![entree("jei", "both", None)]);
    let instance = atelier.options().layout.instance("samflix");

    let crash = mc_instance::crash::parse(TRACE).expect("une exception");
    let lock = verrou(vec![entree("jei", "both", None)]);

    let mut identifiant = None;
    let evenements = sentry::test::with_captured_events(|| {
        identifiant = Some(report_game_error(
            &instance,
            &lock,
            "neoforge-21.1.250",
            &crash,
            Some(1),
        ));
    });

    assert_eq!(evenements.len(), 1, "{evenements:?}");
    assert_ne!(
        identifiant.unwrap(),
        sentry::types::Uuid::nil(),
        "aucun identifiant à donner au joueur"
    );

    // Le contexte joint est ce qu'on demanderait sinon au joueur, question
    // par question.
    let extra = &evenements[0].extra;
    assert_eq!(extra["version"].as_str(), Some("neoforge-21.1.250"));
    assert_eq!(extra["minecraft"].as_str(), Some("1.21.1"));
    assert_eq!(extra["neoforge"].as_str(), Some("21.1.250"));
    assert_eq!(extra["code_sortie"].as_str(), Some("1"));
    assert!(
        extra["mods"].as_str().unwrap().contains("jei"),
        "la liste des mods manque : {extra:?}"
    );

    // Sans code de sortie, l'incident se distingue d'une erreur fatale :
    // autrement, les deux se confondraient dans le tableau de bord.
    let traversees = sentry::test::with_captured_events(|| {
        report_game_error(&instance, &lock, "neoforge-21.1.250", &crash, None);
    });
    assert!(
        !traversees[0].extra.contains_key("code_sortie"),
        "{:?}",
        traversees[0].extra
    );
}

/// Un plantage avec trace devient un incident à part entière : c'est le seul
/// moment où l'on dispose de l'exception, et un joueur ne pensera ni à la
/// trouver ni à la joindre.
#[test]
fn un_plantage_avec_trace_part_comme_incident() {
    let atelier = Atelier::neuf("incident-envoi");
    atelier.pack_installe(vec![entree("jei", "both", None)]);
    let instance = atelier.options().layout.instance("samflix");

    let debut = lancement();
    let logs = instance.game_dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("latest.log"), TRACE).unwrap();

    let evenements = sentry::test::with_captured_events(|| {
        report_game_crash(
            &instance,
            &verrou(vec![entree("jei", "both", None)]),
            "neoforge-21.1.250",
            debut,
            1,
        );
    });

    assert_eq!(evenements.len(), 1, "{evenements:?}");
    let exception = evenements[0]
        .exception
        .values
        .first()
        .expect("la trace est jointe comme exception");
    assert_eq!(exception.ty, "java.lang.module.ResolutionException");
}

/// Sans trace, aucun incident de jeu ne part : il n'y a pas d'exception à
/// regrouper, et un incident vide encombrerait le tableau de bord sans rien
/// apprendre.
#[test]
fn un_plantage_sans_trace_n_envoie_pas_d_incident_de_jeu() {
    let atelier = Atelier::neuf("incident-muet");
    atelier.pack_installe(Vec::new());
    let instance = atelier.options().layout.instance("samflix");

    let evenements = sentry::test::with_captured_events(|| {
        report_game_crash(
            &instance,
            &verrou(Vec::new()),
            "neoforge-21.1.250",
            lancement(),
            1,
        );
    });

    assert!(evenements.is_empty(), "{evenements:?}");
}
