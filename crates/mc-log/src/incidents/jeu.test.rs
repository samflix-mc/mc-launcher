use super::{capture_game_crash, truncate};

/// Trois champs font tout le travail de regroupement chez Sentry : le type de
/// l'exception, qui sépare un plantage d'un autre ; `logger`, qui distingue le
/// jeu du launcher ; et le niveau, qui décide si l'incident est vu. Les perdre
/// ne casse rien ici — cela ne se verrait que dans le tableau de bord, une
/// fois l'incident remonté et confondu avec mille autres.
#[test]
fn un_plantage_du_jeu_porte_de_quoi_le_regrouper() {
    use sentry::protocol::Level;

    let contexte = std::collections::BTreeMap::from([
        ("instance".to_string(), "noah".to_string()),
        ("version".to_string(), "1.21.1".to_string()),
    ]);

    let evenements = sentry::test::with_captured_events(|| {
        capture_game_crash(
            "java.lang.OutOfMemoryError",
            "Java heap space",
            "dernières lignes du journal",
            &contexte,
        );
    });

    assert_eq!(evenements.len(), 1, "un seul incident par plantage");
    let evenement = &evenements[0];
    assert_eq!(evenement.level, Level::Error);
    assert_eq!(evenement.logger.as_deref(), Some("minecraft"));

    let exception = evenement
        .exception
        .values
        .first()
        .expect("la trace est jointe comme exception, et non comme message");
    assert_eq!(exception.ty, "java.lang.OutOfMemoryError");
    assert_eq!(exception.value.as_deref(), Some("Java heap space"));
    // Un module Java n'est pas un module Sentry : le champ reste vide, sans
    // quoi le regroupement suit une hiérarchie qui n'existe pas.
    assert!(exception.module.is_none());

    assert_eq!(evenement.extra["instance"].as_str(), Some("noah"));
    assert_eq!(evenement.extra["version"].as_str(), Some("1.21.1"));
    assert_eq!(
        evenement.extra["journal"].as_str(),
        Some("dernières lignes du journal")
    );
}

/// Un journal de jeu contient le pseudo du joueur et le chemin de son
/// répertoire personnel. Ce qui part chez Sentry passe par les mêmes filtres
/// que ce qui s'écrit dans la console.
#[test]
fn ce_qui_part_est_censure_comme_le_reste() {
    let maison = std::env::var("HOME").expect("HOME est posé");
    let contexte =
        std::collections::BTreeMap::from([("chemin".to_string(), format!("{maison}/parties"))]);

    let evenements = sentry::test::with_captured_events(|| {
        capture_game_crash(
            "java.io.IOException",
            &format!("échec d'écriture dans {maison}/.minecraft"),
            &format!("at {maison}/mods/truc.jar"),
            &contexte,
        );
    });

    let evenement = &evenements[0];
    let tout = format!(
        "{:?} {:?} {:?}",
        evenement.exception.values[0].value, evenement.extra["journal"], evenement.extra["chemin"]
    );
    assert!(
        !tout.contains(&maison),
        "le répertoire personnel n'a pas été masqué : {tout}"
    );
}

#[test]
fn un_extrait_se_tronque_sans_couper_un_caractere() {
    // Un journal de jeu est plein d'accents : trancher au milieu d'un
    // caractère ferait paniquer le rapport de plantage lui-même.
    let texte = "é".repeat(200);
    let borne = truncate(&texte, 101);
    assert!(borne.ends_with('é'));
    // `strip_prefix` et non `trim_start_matches` : celui-ci ne fait rien
    // quand le marqueur manque, si bien que l'assertion passait encore le
    // jour où la troncature cessait d'en poser un.
    let extrait = borne
        .strip_prefix("[…début tronqué…]\n")
        .expect("marqueur de troncature absent");
    assert!(texte.ends_with(extrait));
    // La borne est un nombre d'octets, et la frontière se cherche vers
    // l'avant : reculer d'un caractère au lieu d'avancer rendrait deux octets
    // de plus, ce que « se termine par é » ne voit pas.
    assert_eq!(extrait.len(), 100);
}

/// Un journal se coupe à une fin de ligne, et pas n'importe où : une trace
/// Java qui commence au milieu d'un « at ... » n'apprend rien.
#[test]
fn la_troncature_reprend_au_debut_d_une_ligne() {
    // Trente lignes de dix octets chacune : ce qui doit rester se calcule à
    // la main, et toute erreur d'un octet se voit.
    let texte: String = (0..30).map(|i| format!("ligne-{i:03}\n")).collect();
    assert_eq!(texte.len(), 300);

    // Cent octets demandés : la coupe tombe au début de « ligne-020 », que le
    // passage à la ligne suivante écarte — on garde les neuf dernières.
    let attendu: String = (21..30).map(|i| format!("ligne-{i:03}\n")).collect();
    assert_eq!(
        truncate(&texte, 100),
        format!("[…début tronqué…]\n{attendu}")
    );
}

/// En dessous de la borne, le texte sort tel quel : pas de marqueur, rien de
/// perdu. C'est le cas ordinaire, celui d'un plantage survenu tôt.
#[test]
fn un_texte_plus_court_que_la_borne_est_rendu_intact() {
    let texte = "trois\nlignes\ncourtes\n";
    assert_eq!(truncate(texte, 8_000), texte);
    // Exactement à la borne, rien ne se coupe non plus.
    assert_eq!(truncate(texte, texte.len()), texte);
}
