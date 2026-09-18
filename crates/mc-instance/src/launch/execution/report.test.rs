use super::Outcome;

#[cfg(unix)]
fn status(raw: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(raw)
}

#[cfg(unix)]
#[test]
fn a_normal_exit_does_not_open_an_incident() {
    // Code 0: end screen, or window closed.
    let report = Outcome::from_status(&status(0));
    assert_eq!(report, Outcome::Normal);
    assert!(!report.is_failure());
}

/// Closing the game with Ctrl+C isn't a crash. Confusing it with an error
/// used to fill the incident dashboard on every close.
#[cfg(unix)]
#[test]
fn a_stop_requested_from_outside_is_not_a_crash() {
    // Raw Unix status: the low byte carries the signal. 15 = SIGTERM.
    let report = Outcome::from_status(&status(15));
    assert_eq!(report, Outcome::Interrupted { signal: 15 });
    assert!(!report.is_failure());
}

/// A shell translates a signal into 128 + n. `status.signal()` doesn't see
/// it when the code crosses an intermediary: 143 is a SIGTERM, 130 a
/// Ctrl+C.
#[cfg(unix)]
#[test]
fn a_signal_translated_by_a_shell_is_recognized_too() {
    for (code, signal) in [(143, 15), (130, 2)] {
        let report = Outcome::from_status(&status(code << 8));
        assert_eq!(report, Outcome::Interrupted { signal }, "for code {code}");
        assert!(!report.is_failure());
    }
}

#[cfg(unix)]
#[test]
fn a_stop_on_error_opens_an_incident() {
    let report = Outcome::from_status(&status(1 << 8));
    assert_eq!(report, Outcome::Failed { code: 1 });
    assert!(report.is_failure());
}

/// The upper bound matters: past 192, a code is no longer a translated
/// signal but an exit code the game chose itself.
#[cfg(unix)]
#[test]
fn a_code_outside_the_signal_range_stays_an_error() {
    assert_eq!(
        Outcome::from_status(&status(200 << 8)),
        Outcome::Failed { code: 200 }
    );
    assert_eq!(
        Outcome::from_status(&status(128 << 8)),
        Outcome::Failed { code: 128 }
    );
}

/// A stop for which the system returns no code can't come from any
/// program: the fallback says so with a value no process produces.
/// Returning 1 instead would pass an unexplained stop off as an ordinary
/// game error, and the incident report would go looking for an exception
/// that doesn't exist.
#[test]
fn a_stop_without_a_code_is_not_confused_with_a_game_error() {
    assert_eq!(Outcome::failure(None), Outcome::Failed { code: -1 });
    assert_eq!(Outcome::failure(Some(1)), Outcome::Failed { code: 1 });
}
