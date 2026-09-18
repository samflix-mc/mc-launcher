//! What the bar displays, and above all what it must never display.
//!
//! The bugs targeted here are the ones that don't fail any build and that
//! only show up watching a real install for ten minutes: a bar that goes
//! past a hundred percent, an infinite remaining time, a counter that wraps
//! back around zero.

use super::{Phase, Tracker, in_progress, instant_rate, remaining, smooth};

fn tracker() -> Tracker {
    Tracker::default()
}

#[test]
fn received_bytes_add_up() {
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Received(1_000));
    tracker.download(mc_dl::Progress::Received(2_500));

    assert_eq!(tracker.snapshot().bytes, 3_500);
}

#[test]
fn a_retry_does_not_overflow_the_bar() {
    // A failed attempt mid-way, then resumed: the abandoned bytes are
    // removed, otherwise the bar would count the same file twice.
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Received(800));
    tracker.download(mc_dl::Progress::Lost(800));
    tracker.download(mc_dl::Progress::Received(1_000));

    assert_eq!(tracker.snapshot().bytes, 1_000);
}

#[test]
fn subtracting_more_than_counted_does_not_wrap_around_zero() {
    // On an unsigned integer, an overflowing subtraction gives sixteen
    // exabytes. That's the kind of number that ends up on screen.
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Received(100));
    tracker.download(mc_dl::Progress::Lost(5_000));

    assert_eq!(tracker.snapshot().bytes, 0);
}

#[test]
fn a_file_already_present_moves_the_bar_without_downloading_anything() {
    // The case of a reinstall: without this, the bar would stay at zero
    // from end to end even though everything's already on disk.
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Finished {
        file: "jei.jar",
        state: mc_dl::Fetched::AlreadyPresent,
        bytes: 4_000,
    });

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.bytes, 4_000);
    assert_eq!(snapshot.files, 1);
}

#[test]
fn a_downloaded_file_is_not_counted_twice() {
    // Its bytes already went through `Received`: adding them again on
    // arrival would finish the bar at double the total.
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Received(4_000));
    tracker.download(mc_dl::Progress::Finished {
        file: "jei.jar",
        state: mc_dl::Fetched::Downloaded,
        bytes: 4_000,
    });

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.bytes, 4_000);
    assert_eq!(snapshot.files, 1);
}

#[test]
fn a_new_batch_starts_over_from_zero() {
    // Steps chain — libraries, then assets, then mods — and each announces
    // its own total. Accumulating would make a bar that never advances and
    // a total that means nothing.
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Batch {
        files: 10,
        bytes: 1_000,
    });
    tracker.download(mc_dl::Progress::Received(1_000));
    tracker.download(mc_dl::Progress::Batch {
        files: 2_500,
        bytes: 830_000_000,
    });

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.bytes, 0);
    assert_eq!(snapshot.total, 830_000_000);
    assert_eq!(snapshot.files, 0);
    assert_eq!(snapshot.files_total, 2_500);
}

#[test]
fn the_current_file_is_the_one_that_just_started() {
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Started {
        file: "lwjgl.jar",
        bytes: Some(900),
    });
    tracker.download(mc_dl::Progress::Started {
        file: "jei-1.21.1.jar",
        bytes: None,
    });

    assert_eq!(tracker.snapshot().file.as_deref(), Some("jei-1.21.1.jar"));
}

#[test]
fn phase_and_note_pass_through_unchanged() {
    let tracker = tracker();
    tracker.phase(Phase::Mods);
    tracker.note("128 mods, 43 of them added by resolution");

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.phase, Phase::Mods);
    assert_eq!(
        snapshot.note.as_deref(),
        Some("128 mods, 43 of them added by resolution")
    );
}

#[test]
fn the_instant_rate_is_computed_over_the_interval() {
    assert_eq!(instant_rate(1_000_000, 2.0), 500_000.0);
}

#[test]
fn a_null_interval_does_not_give_infinity() {
    // Two snapshots in the same microsecond: dividing by zero would display
    // "inf B/s", then an integer conversion with undefined behavior.
    assert_eq!(instant_rate(1_000, 0.0), 0.0);
    assert_eq!(instant_rate(1_000, -1.0), 0.0);
}

#[test]
fn smoothing_tends_toward_the_measurement_without_reaching_it_at_once() {
    // The value must move — smoothing at zero would freeze the display —
    // without reaching the measurement immediately, otherwise it smooths
    // nothing.
    let after = smooth(0.0, 1_000.0);
    assert!(after > 0.0 && after < 1_000.0, "{after}");

    // And repeating it, it converges.
    let mut value = 0.0;
    for _ in 0..100 {
        value = smooth(value, 1_000.0);
    }
    assert!((value - 1_000.0).abs() < 1.0, "{value}");
}

#[test]
fn remaining_time_is_derived_from_the_rate() {
    // 500 MB left at 10 MB/s: fifty seconds.
    assert_eq!(remaining(1_000, 500, 10.0), Some(50));
}

#[test]
fn remaining_time_rounds_up() {
    // Truncating would display "0s" during the last second.
    assert_eq!(remaining(1_000, 999, 2.0), Some(1));
}

#[test]
fn without_an_announced_total_no_time_is_promised() {
    // The source doesn't publish sizes: better to display nothing than an
    // invented estimate.
    assert_eq!(remaining(0, 500, 10.0), None);
}

#[test]
fn without_a_rate_no_time_is_promised() {
    // Dividing by zero would give infinity, then some arbitrary integer.
    assert_eq!(remaining(1_000, 500, 0.0), None);
    assert_eq!(remaining(1_000, 500, -1.0), None);
}

#[test]
fn an_exceeded_total_promises_nothing_either() {
    // CurseForge mods without a key count for zero in the total: we then
    // exceed what was announced. Displaying "0s" would suggest it's
    // finished when files remain.
    assert_eq!(remaining(1_000, 1_000, 10.0), None);
    assert_eq!(remaining(1_000, 5_000, 10.0), None);
}

/// The bug observed in a real install: the bar stayed full while mod
/// resolution was still working. The previous batch was complete, the next
/// not yet announced — and a hundred percent, in that interval, is a lie.
#[test]
fn a_complete_batch_no_longer_counts_as_a_download_in_progress() {
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Batch {
        files: 2,
        bytes: 200,
    });
    assert!(tracker.snapshot().active, "the batch just started");

    for _ in 0..2 {
        tracker.download(mc_dl::Progress::Finished {
            file: "jei.jar",
            state: mc_dl::Fetched::Downloaded,
            bytes: 100,
        });
    }

    assert!(!tracker.snapshot().active, "all files are settled");
}

#[test]
fn without_an_announced_batch_nothing_is_in_progress() {
    // The NeoForge installer runs in its JVM without downloading anything:
    // a numbered bar wouldn't make sense.
    assert!(!in_progress(0, 0));
    assert!(in_progress(0, 10));
    assert!(!in_progress(10, 10));
    assert!(!in_progress(11, 10));
}

/// "Ready to play" isn't a step being executed, it's the result of the nine
/// before it. Lighting it up as a step in progress would suggest there's
/// still something to wait for.
#[test]
fn a_done_phase_is_distinct_from_a_phase_still_working() {
    let tracker = tracker();

    tracker.phase(Phase::Mods);
    assert!(!tracker.snapshot().done);

    tracker.finish(Phase::Ready);
    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.phase, Phase::Ready);
    assert!(snapshot.done);
}

/// Without this, the last download's bar would stay displayed full under
/// "Ready to play", as if something were still going on.
#[test]
fn finishing_clears_the_batch_and_the_current_file() {
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Batch {
        files: 3,
        bytes: 300,
    });
    tracker.download(mc_dl::Progress::Started {
        file: "jei.jar",
        bytes: Some(100),
    });
    tracker.download(mc_dl::Progress::Received(300));

    tracker.finish(Phase::Ready);

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.total, 0);
    assert_eq!(snapshot.bytes, 0);
    assert_eq!(snapshot.files_total, 0);
    assert_eq!(snapshot.file, None);
    assert!(!snapshot.active);
}

/// Resuming an install after finishing it must turn the "in progress" state
/// back on: otherwise the path would stay stuck on "Ready".
#[test]
fn resuming_a_phase_cancels_the_done_state() {
    let tracker = tracker();
    tracker.finish(Phase::Ready);

    tracker.phase(Phase::Minecraft);

    assert!(!tracker.snapshot().done);
}

#[test]
fn the_snapshot_serializes_in_camel_case() {
    // Field names are what TypeScript reads.
    let tracker = tracker();
    tracker.download(mc_dl::Progress::Batch {
        files: 3,
        bytes: 99,
    });

    let json = serde_json::to_value(tracker.snapshot()).expect("serialization");
    assert_eq!(json["filesTotal"], 3);
    assert_eq!(json["total"], 99);
    assert_eq!(json["phase"], "signin");
}

/// **Resolution moves the bar without downloading a single byte.**
///
/// It queries the APIs one after another: about thirty seconds for a
/// fifty-mod pack, for a few kilobytes. Without this count,
/// `snapshot()` would return an empty batch — zero files, zero bytes — and
/// the window would show a motionless step indistinguishable from a crash.
#[test]
fn resolution_advances_through_the_request_count() {
    let tracker = Tracker::default();

    tracker.resolution(12, 51);
    let seen = tracker.snapshot();

    assert_eq!((seen.files, seen.files_total), (12, 51));
    assert!(seen.active, "there are still requests to settle");
    // **And the byte total is zero**, which is the truth: nothing
    // measurable is announced. It's this zero that tells the window to rely
    // on the count rather than the weight.
    assert_eq!(seen.total, 0);
}

/// The download batch that follows takes back control of the count.
///
/// Both write to the same counters, and in this order: resolution first,
/// the download second. If the batch didn't overwrite it, the bar would
/// start over from a request count that no longer applies.
#[test]
fn the_download_batch_takes_back_control_from_resolution() {
    let tracker = Tracker::default();
    tracker.resolution(51, 51);

    tracker.download(mc_dl::Progress::Batch {
        files: 8,
        bytes: 840_000,
    });
    let seen = tracker.snapshot();

    assert_eq!((seen.files, seen.files_total), (0, 8));
    assert_eq!(seen.total, 840_000);
}
