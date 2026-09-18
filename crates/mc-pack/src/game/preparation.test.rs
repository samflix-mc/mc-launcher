use super::{Comfort, launch_options};
use mc_instance::launch::QuickPlay;

/// Two settings, and two ways to silently lose them. Without requested
/// memory, the JVM falls back to its default — a quarter of the machine,
/// which isn't enough for a modpack and gives an `OutOfMemoryError` after
/// twenty minutes of play.
#[test]
fn the_requested_memory_is_passed_to_the_game() {
    let options = launch_options(
        &Comfort {
            memory_mb: Some(6144),
            ..Default::default()
        },
        None,
    );
    assert_eq!(options.memory_mb, Some(6144));

    // Absent that, we let the JVM decide: that's the case of a launch
    // without `--memory`.
    assert_eq!(launch_options(&Comfort::default(), None).memory_mb, None);
}

/// Without Quick Play, the game opens on its menu instead of joining the
/// server — and it looks as if the pack doesn't declare one.
#[test]
fn the_server_to_join_becomes_a_quick_play() {
    let options = launch_options(&Comfort::default(), Some("mc.example.com".to_string()));

    match options.quick_play {
        Some(QuickPlay::Multiplayer(host)) => assert_eq!(host, "mc.example.com"),
        other => panic!("the server isn't joined: {other:?}"),
    }

    // Without a target — preproduction has no server —, the menu is the
    // right behavior.
    assert!(
        launch_options(&Comfort::default(), None)
            .quick_play
            .is_none()
    );
}

/// **The resolution and fullscreen make it all the way to the game.**
///
/// Both fields could be REMOVED from the structure without a test flinching:
/// that's what mutation testing showed. The symptom would have been the
/// worst possible — the player sets their window size in Settings, the value
/// is written to `settings.json`, reread, validated, passed along… and
/// ignored at the last link. Nothing in the log, nothing on screen, and a
/// setting that "doesn't work" without anyone knowing where it gets lost.
///
/// `resolution` does more than size the window: it activates
/// `has_custom_resolution` in Mojang's descriptor, which unlocks conditional
/// arguments. Losing it therefore changes the command line, not just the
/// size.
#[test]
fn the_resolution_and_fullscreen_make_it_to_the_game() {
    let options = launch_options(
        &Comfort {
            resolution: Some((1600, 900)),
            fullscreen: true,
            ..Default::default()
        },
        None,
    );

    assert_eq!(options.resolution, Some((1600, 900)));
    assert!(options.fullscreen);
}

/// And defaults really are defaults: nothing imposed on the game.
///
/// The counterpart to the previous test. Setting a resolution nobody asked
/// for would activate `has_custom_resolution` on every machine, and thus
/// arguments Mojang reserves for that case.
#[test]
fn without_a_setting_nothing_is_imposed_on_the_game() {
    let options = launch_options(&Comfort::default(), None);

    assert_eq!(options.resolution, None);
    assert!(!options.fullscreen);
}
