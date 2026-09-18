use super::super::session::{QuickPlay, Session};
use super::{LaunchOptions, active_features, variables};
use std::collections::BTreeMap;
use std::path::Path;

fn table(session: &Session, options: &LaunchOptions) -> BTreeMap<String, String> {
    variables(
        "neoforge-21.1.250",
        "1.21.1",
        Path::new("/game"),
        Path::new("/shared"),
        "17",
        Path::new("/natives"),
        Path::new("/shared/libraries"),
        "/a.jar:/b.jar",
        ":",
        session,
        options,
    )
}

#[test]
fn flags_follow_the_options() {
    let without = active_features(&LaunchOptions::default());
    assert!(without.is_empty());

    let with = active_features(&LaunchOptions {
        quick_play: Some(QuickPlay::Multiplayer("mc.exemple.fr".into())),
        resolution: Some((1280, 720)),
        ..Default::default()
    });
    assert!(with.contains("is_quick_play_multiplayer"));
    assert!(with.contains("has_quick_plays_support"));
    assert!(with.contains("has_custom_resolution"));
    assert!(!with.contains("is_quick_play_singleplayer"));
}

#[test]
fn a_local_world_activates_its_own_flag() {
    let with = active_features(&LaunchOptions {
        quick_play: Some(QuickPlay::Singleplayer("New world".into())),
        ..Default::default()
    });
    assert!(with.contains("is_quick_play_singleplayer"));
    assert!(!with.contains("is_quick_play_multiplayer"));
}

/// Player identity is the part that must not be gotten wrong: a bad
/// `auth_uuid` swaps the inventory, a bad token refuses the connection.
#[test]
fn the_players_identity_fills_the_descriptors_variables() {
    let session = Session::online("Sam", "0123456789ab", "msa-token");
    let v = table(&session, &LaunchOptions::default());

    assert_eq!(v["auth_player_name"], "Sam");
    assert_eq!(v["auth_uuid"], "0123456789ab");
    assert_eq!(v["auth_access_token"], "msa-token");
    assert_eq!(v["user_type"], "msa");
    // Legacy form, still expected by some descriptors.
    assert_eq!(v["auth_session"], "token:msa-token");
}

/// Assets and libraries live in the shared directory; the game directory
/// belongs only to the instance.
#[test]
fn paths_distinguish_the_shared_folder_from_the_instance() {
    let v = table(&Session::offline("Sam", "0123"), &LaunchOptions::default());

    assert_eq!(v["game_directory"], "/game");
    assert_eq!(v["assets_root"], "/shared/assets");
    assert_eq!(v["game_assets"], v["assets_root"]);
    assert_eq!(v["library_directory"], "/shared/libraries");
    assert_eq!(v["natives_directory"], "/natives");
    assert_eq!(v["assets_index_name"], "17");
    assert_eq!(v["classpath"], "/a.jar:/b.jar");
    assert_eq!(v["classpath_separator"], ":");
    assert_eq!(v["version_name"], "neoforge-21.1.250");
    // The base's jar, which NeoForge names in its `ignoreList`.
    assert_eq!(v["primary_jar_name"], "1.21.1.jar");
    assert_eq!(v["launcher_name"], "Helm");
}

/// Quick Play and resolution variables only exist when requested: the
/// descriptor only references them behind a rule.
#[test]
fn optional_variables_stay_absent_when_nothing_requests_them() {
    let v = table(&Session::offline("Sam", "0123"), &LaunchOptions::default());

    assert!(!v.contains_key("quickPlayMultiplayer"));
    assert!(!v.contains_key("quickPlayPath"));
    assert!(!v.contains_key("resolution_width"));
}

#[test]
fn joining_a_server_sets_its_target_and_its_file() {
    let v = table(
        &Session::offline("Sam", "0123"),
        &LaunchOptions {
            quick_play: Some(QuickPlay::Multiplayer("mc.ggy.info:25566".into())),
            resolution: Some((1920, 1080)),
            ..Default::default()
        },
    );

    assert_eq!(v["quickPlayMultiplayer"], "mc.ggy.info:25566");
    assert_eq!(v["quickPlayPath"], "/game/quickPlay.json");
    assert_eq!(v["resolution_width"], "1920");
    assert_eq!(v["resolution_height"], "1080");
}

#[test]
fn opening_a_local_world_sets_its_folder_name() {
    let v = table(
        &Session::offline("Sam", "0123"),
        &LaunchOptions {
            quick_play: Some(QuickPlay::Singleplayer("New world".into())),
            ..Default::default()
        },
    );

    assert_eq!(v["quickPlaySingleplayer"], "New world");
    assert_eq!(v["quickPlayPath"], "/game/quickPlay.json");
    assert!(!v.contains_key("quickPlayMultiplayer"));
}
