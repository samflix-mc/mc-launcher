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
