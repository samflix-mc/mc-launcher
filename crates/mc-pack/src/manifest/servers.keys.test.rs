//! The keys `server_problems` rejects, and why.

use crate::manifest::fixtures::with_key;

#[test]
fn a_bad_server_key_is_flagged() {
    // Without this check, a typo causes nothing: the key matches no
    // environment, the game opens on the menu, and it looks exactly like a
    // pack that declared nothing.
    let problems = with_key("prodution", "mc.ggy.info").server_problems();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("prodution"));
}

#[test]
fn an_alias_is_flagged_because_it_would_never_be_read() {
    // Environment::parse accepts “dev”, but server_for looks for
    // “development”: the entry would pass here and stay unreachable.
    let problems = with_key("dev", "mc-dev.ggy.info").server_problems();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("development"));
}

#[test]
fn a_local_key_is_flagged() {
    let problems = with_key("local", "mc-dev.ggy.info").server_problems();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("development"));
}

#[test]
fn an_empty_host_is_flagged() {
    assert_eq!(with_key("production", "   ").server_problems().len(), 1);
}
