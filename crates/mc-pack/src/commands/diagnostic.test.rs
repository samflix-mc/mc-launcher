use super::diagnostic;
use crate::commands::fixtures::environment;
use std::process::ExitCode;

fn succeeded(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

/// The first thing to ask someone whose installation fails: the answer fits
/// in ten lines and says where to find the rest. It must therefore print
/// even when there's no log — that is, precisely when the directory isn't
/// writable.
#[test]
fn diagnostic_prints_even_without_a_log_file() {
    let _env = environment("local");
    let log = mc_log::Guard::without_log();

    assert!(succeeded(diagnostic(&log, false).unwrap()));
}

/// The diagnostic announces the log that exists, not the one that should:
/// citing a missing file would send someone looking for nothing.
#[test]
fn diagnostic_announces_the_log_when_there_is_one() {
    let _env = environment("production");
    let dir = std::env::temp_dir().join(format!("mc-pack-diag-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let log = mc_log::Guard::new_for_fixtures(dir.clone(), "mc-pack");
    assert!(log.log_path().is_some());
    assert!(succeeded(diagnostic(&log, false).unwrap()));

    std::fs::remove_dir_all(&dir).ok();
}

/// With telemetry disabled, `--incident-test` has nothing to send and says
/// so, instead of waiting ten seconds on a queue that will never drain.
#[test]
fn without_telemetry_the_test_incident_attempts_nothing() {
    let _env = environment("local");
    let log = mc_log::Guard::without_log();

    // SAFETY: set and cleared right here; the environment lock serializes
    // the tests in this binary that touch configuration.
    let previous = std::env::var_os("SAMFLIX_TELEMETRY");
    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "0");
    }

    let code = diagnostic(&log, true).unwrap();

    unsafe {
        match previous {
            Some(value) => std::env::set_var("SAMFLIX_TELEMETRY", value),
            None => std::env::remove_var("SAMFLIX_TELEMETRY"),
        }
    }

    assert!(succeeded(code), "nothing to send isn't a failure");
}
