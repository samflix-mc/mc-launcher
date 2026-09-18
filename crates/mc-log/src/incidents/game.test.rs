use super::{capture_game_crash, truncate};

/// Three fields do all the grouping work at Sentry: the exception type,
/// which separates one crash from another; `logger`, which distinguishes
/// the game from the launcher; and the level, which decides whether the
/// incident is even seen. Losing them breaks nothing here — it would
/// only show up in the dashboard, once the incident is reported and
/// lost among a thousand others.
#[test]
fn a_game_crash_carries_what_groups_it() {
    use sentry::protocol::Level;

    let context = std::collections::BTreeMap::from([
        ("instance".to_string(), "noah".to_string()),
        ("version".to_string(), "1.21.1".to_string()),
    ]);

    let events = sentry::test::with_captured_events(|| {
        capture_game_crash(
            "java.lang.OutOfMemoryError",
            "Java heap space",
            "last lines of the log",
            &context,
        );
    });

    assert_eq!(events.len(), 1, "a single incident per crash");
    let event = &events[0];
    assert_eq!(event.level, Level::Error);
    assert_eq!(event.logger.as_deref(), Some("minecraft"));

    let exception = event
        .exception
        .values
        .first()
        .expect("the trace is attached as an exception, not as a message");
    assert_eq!(exception.ty, "java.lang.OutOfMemoryError");
    assert_eq!(exception.value.as_deref(), Some("Java heap space"));
    // A Java module isn't a Sentry module: the field stays empty,
    // otherwise grouping would follow a hierarchy that doesn't exist.
    assert!(exception.module.is_none());

    assert_eq!(event.extra["instance"].as_str(), Some("noah"));
    assert_eq!(event.extra["version"].as_str(), Some("1.21.1"));
    assert_eq!(event.extra["log"].as_str(), Some("last lines of the log"));
}

/// A game log contains the player's username and their home directory
/// path. What goes out to Sentry passes through the same filters as
/// what's written to the console.
#[test]
fn what_goes_out_is_scrubbed_like_everything_else() {
    let home = std::env::var("HOME").expect("HOME is set");
    let context =
        std::collections::BTreeMap::from([("path".to_string(), format!("{home}/sessions"))]);

    let events = sentry::test::with_captured_events(|| {
        capture_game_crash(
            "java.io.IOException",
            &format!("write failure in {home}/.minecraft"),
            &format!("at {home}/mods/thing.jar"),
            &context,
        );
    });

    let event = &events[0];
    let everything = format!(
        "{:?} {:?} {:?}",
        event.exception.values[0].value, event.extra["log"], event.extra["path"]
    );
    assert!(
        !everything.contains(&home),
        "the home directory wasn't scrubbed: {everything}"
    );
}

#[test]
fn an_excerpt_truncates_without_cutting_a_character() {
    // A game log is full of accented characters: cutting in the middle
    // of one would make the crash report itself panic.
    let text = "é".repeat(200);
    let bound = truncate(&text, 101);
    assert!(bound.ends_with('é'));
    // `strip_prefix` and not `trim_start_matches`: the latter does
    // nothing when the marker is missing, so the assertion would still
    // pass the day the truncation stopped adding one.
    let excerpt = bound
        .strip_prefix("[…start truncated…]\n")
        .expect("truncation marker missing");
    assert!(text.ends_with(excerpt));
    // The bound is a byte count, and the boundary search moves forward:
    // stepping back a character instead of forward would yield two more
    // bytes, which "ends with é" wouldn't catch.
    assert_eq!(excerpt.len(), 100);
}

/// A log gets cut at a line break, not just anywhere: a Java trace
/// starting in the middle of an "at ..." line teaches nothing.
#[test]
fn truncation_resumes_at_the_start_of_a_line() {
    // Thirty lines of ten bytes each: what should remain is computed by
    // hand, and any off-by-one error shows.
    let text: String = (0..30).map(|i| format!("line-{i:04}\n")).collect();
    assert_eq!(text.len(), 300);

    // A hundred bytes requested: the cut lands at the start of
    // "line-0020", which the move to the next line discards — the last
    // nine lines are kept.
    let expected: String = (21..30).map(|i| format!("line-{i:04}\n")).collect();
    assert_eq!(
        truncate(&text, 100),
        format!("[…start truncated…]\n{expected}")
    );
}

/// Below the bound, the text comes out unchanged: no marker, nothing
/// lost. This is the ordinary case, an early crash.
#[test]
fn a_text_shorter_than_the_bound_is_returned_intact() {
    let text = "three\nshort\nlines\n";
    assert_eq!(truncate(text, 8_000), text);
    // Exactly at the bound, nothing gets cut either.
    assert_eq!(truncate(text, text.len()), text);
}
