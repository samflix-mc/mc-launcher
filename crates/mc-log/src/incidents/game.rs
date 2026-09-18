//! The game's crash, reported as an incident in its own right.

use crate::redact::redact;

/// Reports a game crash as an incident in its own right.
///
/// Minecraft crashes in its own JVM: nothing of it reaches the launcher
/// except an exit code. Yet this is the only moment when the stack trace
/// is available, and a player won't think to find it, let alone attach
/// it.
///
/// The Java exception is given to Sentry as a real exception, not as a
/// message: the type then serves as the grouping key. Without that,
/// every game crash — whatever the cause — would form a single "Minecraft
/// stopped" incident, useless for anything.
///
/// The text passes through the same filters as everything else: a game
/// log contains the player's username and absolute paths.
pub fn capture_game_crash(
    exception: &str,
    message: &str,
    excerpt: &str,
    context: &std::collections::BTreeMap<String, String>,
) -> sentry::types::Uuid {
    use sentry::protocol::{Event, Exception, Value};

    // The level isn't written: `Event::default()` already equals `Error`,
    // and repeating it would create a line indistinguishable from its
    // own absence — an immortal mutant, that no test could ever catch.
    // It's the test suite that carries this property: it requires
    // `Level::Error` on the resulting event, and will fail the day the
    // SDK changes its default.
    let mut event = Event {
        // `logger` distinguishes a game crash from a launcher error right
        // away, and they don't get the same handling.
        logger: Some("minecraft".into()),
        exception: vec![Exception {
            ty: exception.to_string(),
            value: Some(redact(message)),
            // Java modules aren't Sentry modules; leaving the field
            // empty avoids a made-up grouping.
            module: None,
            ..Default::default()
        }]
        .into(),
        ..Default::default()
    };

    event.extra.insert(
        "log".into(),
        Value::String(redact(&truncate(excerpt, 8_000))),
    );
    for (key, value) in context {
        event
            .extra
            .insert(key.clone(), Value::String(redact(value)));
    }

    // No flush here: a single session can produce up to five reported
    // exceptions plus its crash, and waiting on the queue each time
    // stalled the launcher for ten seconds per exception whenever the
    // network was down. The wait is requested once, by the caller that
    // announces the identifier — or otherwise by [`Guard`] on shutdown.
    sentry::capture_event(event)
}

/// Bounds a text without cutting in the middle of a line.
fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    // The end of a log is more informative than its start: that's where
    // whatever failed is found.
    //
    // `max` is a byte count, and a game log holds multi-byte
    // characters — accented mod names, "§", "…". Cutting blindly in the
    // middle of a character would make the truncation itself panic, and
    // the launcher would die in the very act of reporting the crash.
    let mut start = text.len() - max;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let from = text[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(start);
    format!("[…start truncated…]\n{}", &text[from..])
}

#[cfg(test)]
#[path = "game.test.rs"]
mod tests;
