//! What can be checked without installing eight hundred megabytes.
//!
//! The sequence itself goes off to Mojang, Microsoft and CurseForge: what's
//! tested here is the translation — the report that fills the tracker, and
//! the few fields the launch reuses from the install.

use super::{Arc, CADENCE, Duration, EVENT_PROGRESS, Phase, ToTheWindow, Tracker};
use mc_pack::Report;

fn report() -> (Arc<Tracker>, ToTheWindow) {
    let tracker = Arc::new(Tracker::default());
    (
        Arc::clone(&tracker),
        ToTheWindow {
            app: None,
            tracker: Arc::clone(&tracker),
        },
    )
}

#[test]
fn an_mc_pack_step_becomes_the_matching_phase() {
    let (tracker, report) = report();

    report.step(mc_pack::Step::Mods);

    assert_eq!(tracker.snapshot().phase, Phase::Mods);
}

#[test]
fn a_note_is_stripped_of_its_terminal_indentation() {
    // `mc-pack` formats for a console: "␣␣128 mods, 43 of them…". The window
    // has its own template, and leading spaces would leave a gap there.
    let (tracker, report) = report();

    report.note("  128 mods, 43 of them added by resolution");

    assert_eq!(
        tracker.snapshot().note.as_deref(),
        Some("128 mods, 43 of them added by resolution")
    );
}

#[test]
fn downloads_feed_the_same_counter() {
    let (tracker, report) = report();

    report.download(mc_dl::Progress::Batch {
        files: 4,
        bytes: 8_000,
    });
    report.download(mc_dl::Progress::Received(2_000));

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.bytes, 2_000);
    assert_eq!(snapshot.total, 8_000);
}

#[test]
fn the_cadence_stays_readable_without_drowning_the_bridge() {
    // Faster, and more messages get sent than the screen can show; slower,
    // and a changing rate becomes jerky. The two bounds are there so a "just
    // to see" tweak doesn't go to an extreme.
    assert!(CADENCE >= Duration::from_millis(100), "{CADENCE:?}");
    assert!(CADENCE <= Duration::from_millis(500), "{CADENCE:?}");
}

#[test]
fn the_event_name_does_not_move() {
    // The TypeScript side listens for this exact string.
    assert_eq!(EVENT_PROGRESS, "cinematic://progress");
}

/// **"Maximized" now means a size, where it used to mean nothing.**
///
/// The comfort passed `None` for this mode, on the belief that the game
/// would ask the window manager for the work area. It doesn't: absent
/// `--width`, Minecraft opens at its own default. The player asked for
/// maximized and got a small window.
#[test]
fn a_maximized_window_takes_the_work_area() {
    let screen = crate::commands::settings::Screen {
        width: 1920,
        height: 1032,
        scale: 1.0,
    };

    assert_eq!(super::logical_size(screen), (1920, 1032));
}

/// On a HiDPI screen the work area comes back doubled, and asking for it
/// verbatim would request a window twice the size of the desktop.
#[test]
fn a_hidpi_screen_is_divided_by_its_scale() {
    let screen = crate::commands::settings::Screen {
        width: 3840,
        height: 2064,
        scale: 2.0,
    };

    assert_eq!(super::logical_size(screen), (1920, 1032));
}

/// A scale of zero cannot happen, and dividing by it would ask for a window
/// of `u32::MAX` pixels. The fallback keeps the physical size, which is
/// wrong by at most a factor of two — where the division would be wrong by
/// four billion.
#[test]
fn an_impossible_scale_does_not_divide_by_zero() {
    let screen = crate::commands::settings::Screen {
        width: 1280,
        height: 720,
        scale: 0.0,
    };

    assert_eq!(super::logical_size(screen), (1280, 720));
}

/// **An absent `options.txt` is not created.**
///
/// Minecraft writes that file on its first run, with a `version:` line it
/// uses to migrate its own data across releases. A partial file invented by
/// the launcher would lose it, and the game would re-apply migrations it had
/// already done. So the first launch of a fresh instance ignores these
/// settings; the second applies them.
#[test]
fn an_absent_options_file_is_left_absent() {
    let dir = std::env::temp_dir().join(format!("mc-app-options-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temporary directory");

    super::apply_video_settings(&dir);

    assert!(
        !dir.join("options.txt").exists(),
        "a file was created where the game had written none"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// What the game already had is kept, in place and in order — only our keys
/// move. A launcher that rewrote the whole file would drop the player's key
/// bindings, their resource packs and their language.
#[test]
fn merging_keeps_what_the_game_wrote() {
    let dir = std::env::temp_dir().join(format!("mc-app-merge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temporary directory");
    let file = dir.join("options.txt");
    std::fs::write(&file, "version:3955\nlang:fr_fr\nrenderDistance:12\n").expect("seed");

    super::apply_video_settings(&dir);

    let after = std::fs::read_to_string(&file).expect("read back");
    assert!(after.contains("version:3955"), "the version line was lost");
    assert!(after.contains("lang:fr_fr"), "a foreign key was lost");
    // The value itself comes from the settings file, which this test does
    // not control; what it defends is that the KEY was taken over in place
    // and not appended a second time.
    assert_eq!(
        after.matches("renderDistance:").count(),
        1,
        "renderDistance was duplicated instead of replaced"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
