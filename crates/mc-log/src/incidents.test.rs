use super::{dsn, flush_incidents, telemetry_active, telemetry_enabled};

#[test]
fn telemetry_can_be_turned_off() {
    let vars = crate::fixtures::variables();

    vars.set("SAMFLIX_TELEMETRY", "0");
    assert!(!telemetry_enabled());
    assert!(dsn().is_none());
    assert!(!telemetry_active());

    vars.set("SAMFLIX_TELEMETRY", "1");
    assert!(telemetry_enabled());

    // The opt-out has to be declared; its absence turns nothing off.
    vars.unset("SAMFLIX_TELEMETRY");
    assert!(telemetry_enabled());
}

/// An opt-out you have to guess isn't one: all four ways of writing "no"
/// must all work, in French as in English.
#[test]
fn every_way_of_writing_no_is_heard() {
    let vars = crate::fixtures::variables();
    for value in ["0", "off", "false", "no", "non", " non "] {
        vars.set("SAMFLIX_TELEMETRY", value);
        assert!(!telemetry_enabled(), "\"{value}\" did not turn it off");
    }
}

#[test]
fn an_empty_dsn_disables_reporting() {
    let vars = crate::fixtures::variables();
    // Telemetry must be active, otherwise the DSN wouldn't even be
    // consulted and the test would pass for the wrong reason.
    vars.unset("SAMFLIX_TELEMETRY");
    vars.set("SENTRY_DSN", "   ");
    assert!(dsn().is_none());
}

/// A DSN that's set replaces the project's: that's what lets incidents
/// from a given deployment be routed elsewhere.
#[test]
fn a_set_dsn_replaces_the_projects() {
    let vars = crate::fixtures::variables();
    vars.unset("SAMFLIX_TELEMETRY");
    vars.set("SENTRY_DSN", " https://key@example.invalid/7 ");
    assert_eq!(dsn().as_deref(), Some("https://key@example.invalid/7"));
}

/// Without a client, nothing was emitted, so nothing gets sent: `false`
/// distinguishes this from a queue that was flushed for real.
#[test]
fn without_a_client_the_queue_is_not_said_to_be_flushed() {
    // **A bare hub, and it is not belt-and-braces.**
    //
    // `sentry::init` binds a client to the MAIN hub, and dropping its
    // `ClientInitGuard` CLOSES that client without unbinding it. So once
    // `a_declared_dsn_opens_a_client` has run anywhere in this binary,
    // `Hub::current()` keeps answering `Some(client)` for the rest of the
    // process — and this test, which asserts the opposite, fails for
    // whatever ran before it rather than for what it checks.
    //
    // A lock would not have helped: this is an ORDER, not a race. Running
    // inside a hub built with no client is what makes the test say what it
    // means, whatever else the binary did first.
    // Reproduce the failure it fixes with `--test-threads=1`.
    let bare = std::sync::Arc::new(sentry::Hub::new(None, Default::default()));

    sentry::Hub::run(bare, || {
        assert!(!flush_incidents(std::time::Duration::from_millis(1)));
    });
}

/// And with a client, the queue flushes and says so. The two callers who
/// wait for this `true` announce an identifier to the player: "logged"
/// doesn't mean "sent", and searching the dashboard for an identifier
/// that never arrived costs more than the wait it was saving.
#[test]
fn with_a_client_the_queue_flushes_and_says_so() {
    sentry::test::with_captured_events(|| {
        assert!(
            flush_incidents(std::time::Duration::from_secs(1)),
            "a client is bound: the queue flushes"
        );
    });
}

/// Without an opt-out or a replacement DSN, it's the launcher's project
/// that receives — so reporting is active. Answering `false` would
/// silently cut off the only path by which a crash on a player's machine
/// reaches us.
#[test]
fn with_nothing_declared_reporting_is_active() {
    let vars = crate::fixtures::variables();
    vars.unset("SAMFLIX_TELEMETRY");
    vars.unset("SENTRY_DSN");

    assert!(telemetry_enabled());
    assert!(telemetry_active());
    assert!(dsn().is_some());
}
