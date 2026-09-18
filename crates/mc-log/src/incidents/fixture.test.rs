use super::send_test_event;

/// Without a Sentry client, nothing goes out — and that's what the
/// return value must say.
///
/// The nuance matters: "an identifier was drawn" is not "the incident
/// actually went out". A diagnostic that announced an identifier
/// nowhere to be found in the dashboard would send people searching for
/// nothing.
#[test]
fn without_a_client_nothing_is_sent() {
    let (_id, sent) = send_test_event();
    assert!(
        !sent,
        "the queue is said to be flushed although no client exists"
    );
}

/// And with a client, the incident goes out, under the announced
/// identifier. That's the pair the diagnostic displays: a null
/// identifier, or a "sent" that isn't, would send someone searching the
/// dashboard for something that isn't there.
#[test]
fn with_a_client_the_incident_goes_out_under_the_announced_identifier() {
    let mut result = None;
    let events = sentry::test::with_captured_events(|| {
        result = Some(send_test_event());
    });

    let (identifier, sent) = result.expect("send_test_event was called");
    assert!(sent, "the queue is not said to be flushed");
    assert_ne!(
        identifier,
        sentry::types::Uuid::nil(),
        "no identifier to search for"
    );

    // Both channels: the message, and the structured log line.
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].event_id, identifier);
}
