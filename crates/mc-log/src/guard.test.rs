use super::Guard;
use super::log::current_log_name;

#[test]
fn without_a_log_there_is_no_path_to_announce() {
    // The case of a read-only directory: we lose the file, and one must not
    // announce a path that doesn't exist.
    assert!(Guard::new(None, None, None).log_path().is_none());
}

/// The path is recomputed on every call, never frozen at startup:
/// `rolling::daily` switches files at midnight UTC, and a run that started
/// before continues in the next one. A frozen path would then designate a
/// file that exists but stops before the failure.
#[test]
fn the_announced_path_carries_the_day_s_date() {
    let dir = std::path::PathBuf::from("/tmp/mc-log-fixture");
    let guard = Guard::new(None, None, Some((dir.clone(), "mc-pack".into())));

    let path = guard.log_path().expect("a log is declared");
    assert_eq!(path, dir.join(current_log_name("mc-pack")));
    assert!(
        path.to_string_lossy().contains("mc-pack.log."),
        "{}",
        path.display()
    );
}
