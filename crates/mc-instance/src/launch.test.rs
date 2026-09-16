use super::*;

fn vars() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("auth_player_name".into(), "Sam".into()),
        ("game_directory".into(), "/jeu".into()),
        ("classpath_separator".into(), ":".into()),
    ])
}

#[test]
fn substitution_simple() {
    assert_eq!(substitute("${auth_player_name}", &vars()), "Sam");
    assert_eq!(
        substitute("--gameDir=${game_directory}", &vars()),
        "--gameDir=/jeu"
    );
}

#[test]
fn plusieurs_variables_dans_un_argument() {
    // Le module path de NeoForge en enchaîne une dizaine.
    let rendu = substitute("a${classpath_separator}b${classpath_separator}c", &vars());
    assert_eq!(rendu, "a:b:c");
}

#[test]
fn une_variable_inconnue_reste_visible() {
    // La vider décalerait les arguments suivants sans rien signaler.
    assert_eq!(substitute("${inconnue}", &vars()), "${inconnue}");
}

#[test]
fn un_texte_sans_variable_est_intact() {
    assert_eq!(substitute("--add-modules", &vars()), "--add-modules");
}

#[test]
fn une_accolade_non_fermee_ne_fait_pas_paniquer() {
    assert_eq!(substitute("${tronque", &vars()), "${tronque");
}

#[test]
fn la_cle_de_bibliotheque_ignore_la_version() {
    // C'est ce qui permet de voir qu'une bibliothèque en remplace une autre.
    assert_eq!(
        library_key("com.google.guava:guava:32.1.2-jre"),
        "com.google.guava:guava"
    );
    assert_eq!(
        library_key("com.google.guava:guava:31.0-jre"),
        library_key("com.google.guava:guava:32.1.2-jre")
    );
}

#[test]
fn le_classifier_distingue_deux_bibliotheques() {
    // lwjgl et lwjgl:natives-linux sont deux fichiers, tous deux nécessaires.
    assert_ne!(
        library_key("org.lwjgl:lwjgl:3.3.3"),
        library_key("org.lwjgl:lwjgl:3.3.3:natives-linux")
    );
}

#[test]
fn les_drapeaux_suivent_les_options() {
    let sans = active_features(&LaunchOptions::default());
    assert!(sans.is_empty());

    let avec = active_features(&LaunchOptions {
        quick_play: Some(QuickPlay::Multiplayer("mc.exemple.fr".into())),
        resolution: Some((1280, 720)),
        ..Default::default()
    });
    assert!(avec.contains("is_quick_play_multiplayer"));
    assert!(avec.contains("has_quick_plays_support"));
    assert!(avec.contains("has_custom_resolution"));
    assert!(!avec.contains("is_quick_play_singleplayer"));
}

#[test]
fn une_session_hors_ligne_porte_un_jeton_non_vide() {
    // Le jeu exige l'argument ; une chaîne vide casse l'analyse.
    let session = Session::offline("Sam", "uuid");
    assert!(!session.token.is_empty());
    assert_eq!(session.user_type, "legacy");
}
