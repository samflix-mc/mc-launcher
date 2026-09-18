//! Scrubbing everything headed for Sentry.

use crate::redact::redact;

/// Scrubs an event end to end.
pub(crate) fn scrub_event(event: &mut sentry::protocol::Event<'static>) {
    if let Some(message) = event.message.take() {
        event.message = Some(redact(&message));
    }
    for exception in &mut event.exception.values {
        exception.value = exception.value.as_deref().map(redact);
        scrub_stacktrace(exception.stacktrace.as_mut());
        scrub_stacktrace(exception.raw_stacktrace.as_mut());
    }
    // `attach_stacktrace` has the current thread's stack attached by an
    // SDK integration, and integrations run before `before_send`. An
    // event without an exception — a `capture_message`, a game crash —
    // therefore only exposes its build paths through here, alongside the
    // loop that scrubs them.
    for thread in &mut event.threads.values {
        scrub_stacktrace(thread.stacktrace.as_mut());
        scrub_stacktrace(thread.raw_stacktrace.as_mut());
    }
    scrub_stacktrace(event.stacktrace.as_mut());
    for value in event.extra.values_mut() {
        scrub_value(value);
    }
    // The fields of a `tracing::error!` don't land in `extra`:
    // `sentry-tracing` files them under the "Rust Tracing Fields"
    // context. Without this pass, an `error = ?error` would go out
    // unchanged — the richest channel of all, and the only one scrubbing
    // would have let slip through.
    for context in event.contexts.values_mut() {
        if let sentry::protocol::Context::Other(fields) = context {
            for value in fields.values_mut() {
                scrub_value(value);
            }
        }
    }
    for tag in event.tags.values_mut() {
        *tag = redact(tag);
    }
}

/// Scrubs the paths in a call stack.
///
/// An absolute path carries the account name of whoever compiled it, and
/// a captured variable carries whatever it carries.
fn scrub_stacktrace(stacktrace: Option<&mut sentry::protocol::Stacktrace>) {
    let Some(stacktrace) = stacktrace else {
        return;
    };
    for frame in &mut stacktrace.frames {
        frame.filename = frame.filename.as_deref().map(redact);
        frame.abs_path = frame.abs_path.as_deref().map(redact);
        for value in frame.vars.values_mut() {
            scrub_value(value);
        }
    }
}

/// Scrubs a structured log attribute.
///
/// The fields of a `tracing` event become attributes: a
/// `tracing::info!(url = %url, ...)` exposes them as-is. Only strings can
/// carry a secret; numbers and booleans are left intact, since they stay
/// useful for sorting and filtering.
pub(crate) fn scrub_log_attribute(attribute: &mut sentry::protocol::LogAttribute) {
    use sentry::protocol::Value;
    if let Value::String(text) = &attribute.0 {
        attribute.0 = Value::String(redact(text));
    }
}

/// Recursively scrubs a JSON value.
pub(crate) fn scrub_value(value: &mut sentry::protocol::Value) {
    use sentry::protocol::Value;
    match value {
        Value::String(text) => *text = redact(text),
        Value::Array(items) => items.iter_mut().for_each(scrub_value),
        Value::Object(fields) => fields.values_mut().for_each(scrub_value),
        _ => {}
    }
}

#[cfg(test)]
#[path = "scrub.test.rs"]
mod tests;
