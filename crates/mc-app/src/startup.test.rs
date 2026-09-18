use super::{GUARD_DELAY, MINIMUM_DURATION, TRANSITION_DONE, remaining_wait};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// A front faster than the floor is HELD, for exactly the missing time.
///
/// This is the case that motivated the rule: measured on Sam's machine, the
/// front signaled after about five hundred milliseconds, and the splash
/// screen passed too fast to be read — it read as a flicker, not as a
/// startup.
///
/// The waits are expressed RELATIVE to `MINIMUM_DURATION` and not as raw
/// numbers. The floor is a comfort setting tuned by eye — it's already
/// moved once — and hardcoded numbers would force rewriting this test on
/// every adjustment. A test that needs repairing to change a comfort value
/// ends up discouraging anyone from changing it.
#[test]
fn a_fast_front_is_held_for_the_missing_time() {
    for elapsed in [
        Duration::from_millis(0),
        Duration::from_millis(100),
        Duration::from_millis(500),
        MINIMUM_DURATION / 2,
    ] {
        assert_eq!(
            remaining_wait(elapsed),
            MINIMUM_DURATION - elapsed,
            "elapsed {elapsed:?}"
        );
        // And the property that really matters: the splash screen holds
        // exactly the floor, whatever time the front took.
        assert_eq!(elapsed + remaining_wait(elapsed), MINIMUM_DURATION);
    }
}

/// A front slower than the floor does NOT wait.
///
/// Adding the floor to the real time would turn an already slow startup
/// into an even slower one — and it's precisely on slow machines that
/// nothing should be added.
#[test]
fn a_slow_front_does_not_wait() {
    assert!(remaining_wait(MINIMUM_DURATION).is_zero());
    assert!(remaining_wait(Duration::from_secs(3)).is_zero());
    // And above all: no overflow. `saturating_sub` and not a plain
    // subtraction, which would panic on `Duration` as soon as the elapsed
    // time exceeds the floor — that is, on every slow machine, and only on
    // those.
    assert!(remaining_wait(Duration::from_secs(600)).is_zero());
}

/// At the exact bound, nothing more is waited for.
///
/// The bound ± 1 test that the repo's conventions require of every
/// comparison: without it, a `<` turned into a `<=` — or the reverse —
/// would survive.
#[test]
fn at_the_exact_bound_and_around_it() {
    assert_eq!(
        remaining_wait(MINIMUM_DURATION - Duration::from_millis(1)),
        Duration::from_millis(1)
    );
    assert!(remaining_wait(MINIMUM_DURATION).is_zero());
    assert!(remaining_wait(MINIMUM_DURATION + Duration::from_millis(1)).is_zero());
}

/// The two bounds don't cross.
///
/// A floor longer than the ceiling would make the guard wait for nothing,
/// and the window would show after the delay meant to save it. That's
/// absurd, and it's the kind of absurdity a comfort setting ends up
/// producing.
#[test]
fn the_floor_stays_under_the_ceiling() {
    assert!(
        MINIMUM_DURATION < GUARD_DELAY,
        "the splash screen's floor exceeds the guard delay"
    );
}

/// The floor is bounded on both sides, and for two opposite reasons.
#[test]
fn the_floor_is_bounded_on_both_sides() {
    assert!(
        MINIMUM_DURATION >= Duration::from_millis(800),
        "too short: the screen would read as a flicker"
    );
    assert!(
        MINIMUM_DURATION <= Duration::from_secs(3),
        "too long: we'd be making people wait for the sake of it"
    );
}

/// The guard delay is longer than any real startup, and shorter than what a
/// player would watch without understanding.
#[test]
fn the_guard_delay_is_bounded_on_both_sides() {
    assert!(
        GUARD_DELAY.as_secs() >= 5,
        "too short: a first launch on a slow disk would be cut off"
    );
    assert!(
        GUARD_DELAY.as_secs() <= 20,
        "too long: nobody watches a frozen screen for twenty seconds"
    );
}

/// The flag starts false: without that, the very first transition would be
/// ignored and the window would never show.
///
/// This test only earns its place because it's the only one in this
/// binary that reads it: `complete` needs an `AppHandle`, which needs a
/// Tauri application, which needs a display server. The rest is tested by
/// running the launcher — see the acceptance test in `docs/interface.md`.
#[test]
fn the_transition_is_not_done_at_the_start() {
    assert!(!TRANSITION_DONE.load(Ordering::Acquire));
}

/// **The rule that cost two overlapping sign-in windows.**
///
/// The front may find out, while the splash screen holds its two seconds,
/// that there's no session: the sign-in window is then already open and
/// the main one already hidden. Closing the splash screen by showing the
/// main one made it reappear ON TOP, on a sign-in page that lives
/// elsewhere — two windows, one of them unusable.
///
/// The guard had been written in the design comment and never in the
/// code. This test therefore covers the only thing that could still forget
/// it.
#[test]
fn the_main_window_does_not_show_during_sign_in() {
    assert!(
        super::should_show_main(false),
        "with no sign-in in progress, the main window takes over"
    );
    assert!(
        !super::should_show_main(true),
        "during sign-in, the main window stays hidden"
    );
}
