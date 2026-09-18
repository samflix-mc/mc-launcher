use super::{ended, running_pid, started, stop_order};

/// **Zero means "no game session", and nothing else.**
///
/// The number left behind by a finished session would be reused by the
/// system for another program — and the "stop" button would then kill that
/// program instead. That's `ended`'s reason for existing.
#[test]
fn the_number_is_set_and_read_back() {
    ended();
    assert_eq!(running_pid(), None);

    started(4242);
    assert_eq!(running_pid(), Some(4242));

    ended();
    assert_eq!(running_pid(), None);
}

/// The stop order, as the system expects it.
///
/// `-KILL` and not `-TERM`: the JVM installs handlers for the latter, and a
/// frozen process — the only case this button is for — never runs them.
/// It's the kind of detail a review lets slide and that you only discover
/// with a stuck game in front of you.
#[test]
fn the_stop_order_is_without_ceremony() {
    let (program, args) = stop_order(1234);

    if cfg!(windows) {
        assert_eq!(program, "taskkill");
        assert!(args.contains(&"/F".to_string()));
        assert!(args.contains(&"/T".to_string()));
    } else {
        assert_eq!(program, "kill");
        assert_eq!(args, vec!["-KILL".to_string(), "1234".to_string()]);
    }
    assert!(args.contains(&"1234".to_string()));
}
