use super::options;

/// A token that looks close enough for scrubbing to recognize it.
const TOKEN: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJSMeKKF2QT4f";

#[test]
fn nothing_identifying_goes_out_by_default() {
    let options = options();

    // The process holds Microsoft, Xbox Live and Minecraft tokens:
    // Sentry's own docs suggest the opposite, and that's exactly what we
    // don't want.
    assert!(
        !options.send_default_pii,
        "send_default_pii must stay false"
    );

    // Without this empty string, the SDK fills in the machine's
    // hostname — on a player's machine, an identifying piece of data
    // that says nothing about the crash.
    assert_eq!(options.server_name.as_deref(), Some(""));
}

#[test]
fn the_release_names_the_launcher_and_not_the_logging_crate() {
    // `release_name!()` would return "mc-log@…" for every binary, which
    // would make it impossible to tell one mc-pack version from another.
    let release = options().release.expect("a release is declared");
    assert!(
        release.starts_with("mc-launcher@"),
        "unexpected release: {release}"
    );
}

#[test]
fn the_environment_is_the_one_that_was_declared() {
    // Both reads must land on the same declaration: without this lock,
    // the `environment::resolution` suite could set its value between
    // them, and this test would fail for a reason that isn't its own.
    let _guard = crate::fixtures::variables();
    assert_eq!(
        options().environment.as_deref(),
        Some(crate::environment::current().as_str())
    );
}

/// Two seconds, not ten: this budget is paid by every command on
/// shutdown, including an offline `verify` that has nothing to send.
#[test]
fn the_shutdown_wait_stays_short() {
    assert!(options().shutdown_timeout <= std::time::Duration::from_secs(2));
    // The stacktrace is attached even without an exception — it's what
    // makes a `capture_message` actionable.
    assert!(options().attach_stacktrace);
}

#[test]
fn an_event_is_scrubbed_before_it_goes_out() {
    let before = options().before_send.expect("a before_send filter is set");

    let event = sentry::protocol::Event {
        message: Some(format!("failure with access_token={TOKEN}")),
        ..Default::default()
    };

    let sent = before(event).expect("the event isn't dropped, only scrubbed");
    let message = sent.message.expect("the message survives");
    assert!(!message.contains("eyJhbGci"), "token in clear: {message}");
    assert!(message.contains("failure with"), "context is lost");
}

#[test]
fn a_breadcrumb_is_scrubbed_too() {
    let before = options()
        .before_breadcrumb
        .expect("a before_breadcrumb filter is set");

    let mut crumb = sentry::protocol::Breadcrumb {
        message: Some(format!("request with access_token={TOKEN}")),
        ..Default::default()
    };
    crumb.data.insert(
        "header".into(),
        sentry::protocol::Value::String(format!("Bearer {TOKEN}")),
    );

    let sent = before(crumb).expect("the breadcrumb isn't dropped");
    assert!(!sent.message.unwrap().contains("eyJhbGci"));
    let header = sent.data.get("header").unwrap().as_str().unwrap();
    assert!(!header.contains("eyJhbGci"), "header in clear: {header}");
}

/// Structured logs travel a separate channel: `before_send` never sees
/// them. Without this second filter, scrubbing would be bypassed by the
/// chattiest channel of all.
#[test]
fn a_structured_log_is_scrubbed_and_loses_the_machines_hostname() {
    let before = options()
        .before_send_log
        .expect("a before_send_log filter is set");

    let mut attributes = sentry::protocol::Map::new();
    attributes.insert(
        "server.address".into(),
        sentry::protocol::LogAttribute::from("sams-machine"),
    );
    attributes.insert(
        "token".into(),
        sentry::protocol::LogAttribute::from(TOKEN.to_string()),
    );
    attributes.insert("attempts".into(), sentry::protocol::LogAttribute::from(3));

    let log = sentry::protocol::Log {
        level: sentry::protocol::LogLevel::Info,
        body: format!("exchange succeeded access_token={TOKEN}"),
        trace_id: None,
        timestamp: std::time::SystemTime::UNIX_EPOCH,
        severity_number: None,
        attributes,
    };

    let sent = before(log).expect("the log isn't dropped");
    assert!(!sent.body.contains("eyJhbGci"), "body: {}", sent.body);
    assert!(
        !sent.attributes.contains_key("server.address"),
        "the player's machine hostname went out with the log"
    );
    let token = &sent.attributes.get("token").unwrap().0;
    assert!(!token.as_str().unwrap().contains("eyJhbGci"));
    // Numbers stay intact: they're used for sorting and carry nothing.
    assert_eq!(sent.attributes.get("attempts").unwrap().0, 3);
}

/// The client only opens if telemetry is active — but then it must
/// actually open. Returning `None` here would silently cut off
/// reporting: the launcher would keep going, logs would be written, and
/// not a single incident would ever arrive.
#[test]
fn a_declared_dsn_opens_a_client() {
    let vars = crate::fixtures::variables();
    vars.unset("SAMFLIX_TELEMETRY");
    // A syntactically valid DSN that goes nowhere: the client opens, and
    // whatever it tries to send won't get past the network.
    vars.set("SENTRY_DSN", "https://key@example.invalid/7");

    let guard = super::init_sentry("mc-test");
    assert!(guard.is_some(), "no client opened for a declared DSN");

    // And nothing opens when the opt-out is set: both halves of the
    // decision are checked together, otherwise one could mask the other.
    drop(guard);
    vars.set("SAMFLIX_TELEMETRY", "0");
    assert!(super::init_sentry("mc-test").is_none());
}
