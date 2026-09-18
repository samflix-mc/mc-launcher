use super::{current_log_name, log_dir, purge_old_logs, retention};

/// Two weeks, said in seconds: the only unit `Duration` knows, and the one
/// where the mistake doesn't show.
#[test]
fn logs_are_kept_for_two_weeks() {
    assert_eq!(retention(), std::time::Duration::from_secs(14 * 86_400));
}

/// A working directory specific to this test.
fn folder(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mc-log-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&path).ok();
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Pushes a file's last-write time back.
fn age(path: &std::path::Path, days: u64) {
    let when = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 24 * 3600);
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(when))
        .unwrap();
}

#[test]
fn logs_live_under_the_data_directory() {
    assert!(log_dir().ends_with("logs"));
    assert!(log_dir().starts_with(mc_paths::current().data));
}

#[test]
fn the_announced_path_is_the_one_the_appender_opens() {
    // This is the file a player is asked to attach: naming it without its
    // date sent them to a missing file. The name is computed on our side,
    // so it's the appender itself that must attest it — if tracing-appender
    // changes its format, this test fails instead of the launcher silently
    // going back to pointing at a phantom file.
    let dir = folder("appender");

    let _appender = tracing_appender::rolling::daily(&dir, "mc-pack.log");
    let announced = dir.join(current_log_name("mc-pack"));

    let exists = announced.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(exists, "{} does not exist", announced.display());
}

/// Fourteen days: enough for a player to find the trace of an incident from
/// the past week, not so much that the directory grows without bound.
#[test]
fn logs_that_are_too_old_disappear_and_the_others_stay() {
    let dir = folder("purge");

    let old = dir.join("mc-pack.log.2020-01-01");
    let recent = dir.join("mc-pack.log.2026-09-17");
    std::fs::write(&old, b"old").unwrap();
    std::fs::write(&recent, b"recent").unwrap();
    age(&old, 30);

    purge_old_logs(&dir);

    let old_exists = old.exists();
    let recent_exists = recent.exists();
    std::fs::remove_dir_all(&dir).ok();

    assert!(!old_exists, "a thirty-day-old log survived");
    assert!(recent_exists, "a log from today was deleted");
}

/// The purge only touches logs. The directory can hold something else — a
/// screenshot, a note left by a player — and deleting it would be an
/// unpleasant surprise.
#[test]
fn the_purge_leaves_what_is_not_a_log() {
    let dir = folder("purge-other");

    let foreign = dir.join("capture.png");
    std::fs::write(&foreign, b"png").unwrap();
    age(&foreign, 30);

    purge_old_logs(&dir);

    let survives = foreign.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(survives, "a foreign file was deleted");
}

/// A missing directory isn't an error: that's the case on the very first
/// launch, right before the file layer creates it.
#[test]
fn purging_a_missing_directory_does_nothing() {
    purge_old_logs(&std::env::temp_dir().join("mc-log-directory-that-does-not-exist"));
}
