use super::{scrub_event, scrub_log_attribute, scrub_value};

#[test]
fn a_structured_log_attribute_is_scrubbed() {
    use sentry::protocol::{LogAttribute, Value};

    // Structured logs go through `before_send_log`, not `before_send`:
    // this filter is the only one that sees them.
    let mut attribute = LogAttribute(Value::String(
        "access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
    ));
    scrub_log_attribute(&mut attribute);
    match &attribute.0 {
        Value::String(text) => {
            assert!(!text.contains("eyJhbGci"));
            assert!(text.contains("[secret]"));
        }
        other => panic!("unexpected type: {other:?}"),
    }
}

#[test]
fn a_numeric_attribute_stays_usable() {
    use sentry::protocol::{LogAttribute, Value};

    // Sizes, durations, HTTP codes: nothing to scrub, and they're used
    // for sorting.
    let mut attribute = LogAttribute(Value::from(2155935));
    scrub_log_attribute(&mut attribute);
    assert_eq!(attribute.0, Value::from(2155935));
}

#[test]
fn the_stack_attached_to_an_event_without_an_exception_is_scrubbed() {
    use sentry::protocol::{Event, Frame, Stacktrace, Thread};

    // `attach_stacktrace` attaches the current thread's stack, outside
    // any exception: it's the only place it arrives for a
    // `capture_message` or a game crash.
    let mut event = Event::default();
    event.threads.values.push(Thread {
        stacktrace: Some(Stacktrace {
            frames: vec![Frame {
                abs_path: Some(
                    "call with token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
                ),
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    });

    scrub_event(&mut event);

    let path = event.threads.values[0].stacktrace.as_ref().unwrap().frames[0]
        .abs_path
        .clone()
        .unwrap();
    assert!(!path.contains("eyJhbGci"), "token in clear: {path}");
    assert!(path.contains("[secret]"));
}

#[test]
fn an_events_fields_are_scrubbed() {
    use sentry::protocol::{Context, Event, Value};

    // `sentry-tracing` doesn't fill `extra`: the fields of a
    // `tracing::error!` land in this context, and only this one.
    let mut event = Event::default();
    event.contexts.insert(
        "Rust Tracing Fields".into(),
        Context::Other(
            [(
                "error".to_string(),
                Value::String(
                    "GET https://api/x?token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
                ),
            )]
            .into_iter()
            .collect(),
        ),
    );

    scrub_event(&mut event);

    let rendered = format!("{:?}", event.contexts);
    assert!(
        !rendered.contains("eyJhbGci"),
        "token sent in clear: {rendered}"
    );
    assert!(rendered.contains("[secret]"));
}

/// A secret isn't always at the root: the variables of a call stack and
/// the contexts of an event are objects containing lists containing
/// objects. Scrubbing therefore has to descend, and not descending
/// breaks nothing visible — it just lets the token slip through one
/// level down.
#[test]
fn scrubbing_descends_into_lists_and_objects() {
    use sentry::protocol::Value;

    let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ";
    let mut value = Value::Array(vec![
        Value::String(format!("access_token={token}")),
        Value::Object(
            [(
                "header".to_string(),
                Value::Array(vec![Value::String(format!(
                    "Authorization: Bearer {token}"
                ))]),
            )]
            .into_iter()
            .collect(),
        ),
        // What isn't text passes through intact: numbers and booleans
        // are used for sorting, and have nothing to hide.
        Value::from(42),
    ]);

    scrub_value(&mut value);

    let rendered = format!("{value:?}");
    assert!(!rendered.contains("eyJhbGci"), "token in clear: {rendered}");
    // The first level, then the one hiding two levels down.
    assert!(rendered.contains("access_token=[secret]"), "{rendered}");
    assert!(rendered.contains("Authorization: [secret]"), "{rendered}");
    assert!(rendered.contains("42"), "{rendered}");
}
