use super::super::session::{QuickPlay, Session};
use super::{LaunchOptions, active_features, variables};
use std::collections::BTreeMap;
use std::path::Path;

fn table(session: &Session, options: &LaunchOptions) -> BTreeMap<String, String> {
    variables(
        "neoforge-21.1.250",
        "1.21.1",
        Path::new("/jeu"),
        Path::new("/partage"),
        "17",
        Path::new("/natives"),
        Path::new("/partage/libraries"),
        "/a.jar:/b.jar",
        ":",
        session,
        options,
    )
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
fn un_monde_local_active_son_propre_drapeau() {
    let avec = active_features(&LaunchOptions {
        quick_play: Some(QuickPlay::Singleplayer("Nouveau monde".into())),
        ..Default::default()
    });
    assert!(avec.contains("is_quick_play_singleplayer"));
    assert!(!avec.contains("is_quick_play_multiplayer"));
}

/// L'identité du joueur est la partie qu'on ne peut pas se tromper : un
/// mauvais `auth_uuid` change d'inventaire, un mauvais jeton refuse la
/// connexion.
#[test]
fn l_identite_du_joueur_remplit_les_variables_du_descripteur() {
    let session = Session::online("Sam", "0123456789ab", "jeton-msa");
    let v = table(&session, &LaunchOptions::default());

    assert_eq!(v["auth_player_name"], "Sam");
    assert_eq!(v["auth_uuid"], "0123456789ab");
    assert_eq!(v["auth_access_token"], "jeton-msa");
    assert_eq!(v["user_type"], "msa");
    // Forme héritée, encore attendue par certains descripteurs.
    assert_eq!(v["auth_session"], "token:jeton-msa");
}

/// Assets et bibliothèques vivent dans le répertoire partagé ; le répertoire de
/// jeu n'appartient qu'à l'instance.
#[test]
fn les_chemins_distinguent_le_partage_de_l_instance() {
    let v = table(&Session::offline("Sam", "0123"), &LaunchOptions::default());

    assert_eq!(v["game_directory"], "/jeu");
    assert_eq!(v["assets_root"], "/partage/assets");
    assert_eq!(v["game_assets"], v["assets_root"]);
    assert_eq!(v["library_directory"], "/partage/libraries");
    assert_eq!(v["natives_directory"], "/natives");
    assert_eq!(v["assets_index_name"], "17");
    assert_eq!(v["classpath"], "/a.jar:/b.jar");
    assert_eq!(v["classpath_separator"], ":");
    assert_eq!(v["version_name"], "neoforge-21.1.250");
    // Le jar du socle, que NeoForge nomme dans son `ignoreList`.
    assert_eq!(v["primary_jar_name"], "1.21.1.jar");
    assert_eq!(v["launcher_name"], "Helm");
}

/// Les variables de Quick Play et de résolution n'existent que lorsqu'elles
/// sont demandées : le descripteur ne les référence que derrière une règle.
#[test]
fn les_variables_facultatives_restent_absentes_quand_rien_ne_les_demande() {
    let v = table(&Session::offline("Sam", "0123"), &LaunchOptions::default());

    assert!(!v.contains_key("quickPlayMultiplayer"));
    assert!(!v.contains_key("quickPlayPath"));
    assert!(!v.contains_key("resolution_width"));
}

#[test]
fn rejoindre_un_serveur_pose_sa_cible_et_son_fichier() {
    let v = table(
        &Session::offline("Sam", "0123"),
        &LaunchOptions {
            quick_play: Some(QuickPlay::Multiplayer("mc.ggy.info:25566".into())),
            resolution: Some((1920, 1080)),
            ..Default::default()
        },
    );

    assert_eq!(v["quickPlayMultiplayer"], "mc.ggy.info:25566");
    assert_eq!(v["quickPlayPath"], "/jeu/quickPlay.json");
    assert_eq!(v["resolution_width"], "1920");
    assert_eq!(v["resolution_height"], "1080");
}

#[test]
fn ouvrir_un_monde_local_pose_son_nom_de_dossier() {
    let v = table(
        &Session::offline("Sam", "0123"),
        &LaunchOptions {
            quick_play: Some(QuickPlay::Singleplayer("Nouveau monde".into())),
            ..Default::default()
        },
    );

    assert_eq!(v["quickPlaySingleplayer"], "Nouveau monde");
    assert_eq!(v["quickPlayPath"], "/jeu/quickPlay.json");
    assert!(!v.contains_key("quickPlayMultiplayer"));
}
