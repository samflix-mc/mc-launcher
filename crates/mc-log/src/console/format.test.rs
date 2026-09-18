use super::ConsoleFormat;
use std::sync::{Arc, Mutex};

/// A shared writer, to read back what the console would have shown.
#[derive(Clone)]
struct Buffer(Arc<Mutex<Vec<u8>>>);

impl Buffer {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn read(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl std::io::Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl tracing_subscriber::fmt::MakeWriter<'_> for Buffer {
    type Writer = Buffer;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

/// Emits `body`'s events into a console formatted by [`ConsoleFormat`], and
/// returns what came out of it.
fn console(body: impl FnOnce()) -> String {
    let buffer = Buffer::new();
    let subscriber = tracing_subscriber::fmt()
        .event_format(ConsoleFormat::new())
        .with_writer(buffer.clone())
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::with_default(subscriber, body);
    buffer.read()
}

/// From `info` on, the message stands on its own. Repeating the fields it
/// already carries would double the line without teaching anything new —
/// they stay in the file and in Sentry, where they're used to filter.
#[test]
fn from_info_on_only_the_message_shows() {
    let output = console(|| {
        tracing::info!(mods = 7, duration_ms = 1234, "instance installed");
    });

    assert!(
        output.contains("instance installed"),
        "message missing: {output}"
    );
    assert!(
        !output.contains("duration_ms"),
        "the fields drown out the line: {output}"
    );
}

/// At `debug`, the fields *are* the information: the message is just a
/// label above them.
#[test]
fn at_debug_the_fields_are_the_information() {
    let output = console(|| {
        tracing::debug!(slug = "jei", version = "19.51.0", "mod retained");
    });

    assert!(output.contains("jei"), "fields missing: {output}");
    assert!(output.contains("19.51.0"), "fields missing: {output}");
}

/// Time elapsed since startup, not the absolute time: knowing a step took
/// 4.2s is informative, knowing it was 01:18:38 isn't.
#[test]
fn each_line_carries_the_elapsed_time_and_its_level() {
    let output = console(|| {
        tracing::warn!("key rejected");
    });

    let line = output.lines().next().expect("at least one line");
    assert!(line.contains('s'), "no duration: {line}");
    assert!(line.contains("WARN"), "no level: {line}");
    // Two decimals are enough to place a step; the default format's
    // nanosecond only serves to lengthen the line.
    assert!(
        line.split('s').next().unwrap().contains('.'),
        "duration without a decimal: {line}"
    );
}

/// The clock is captured once at construction. Recreating it on every line
/// would show zero everywhere — that was the first attempt.
#[test]
fn the_clock_does_not_restart_from_zero_on_every_line() {
    let format = ConsoleFormat::new();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let buffer = Buffer::new();
    let subscriber = tracing_subscriber::fmt()
        .event_format(format)
        .with_writer(buffer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || tracing::info!("later"));

    let output = buffer.read();
    assert!(
        !output.trim_start().starts_with("0.00s"),
        "the clock restarted from zero: {output}"
    );
}

/// A message can reach the visitor as a string rather than through its
/// `Debug` impl: that's the case as soon as it's passed as a named field,
/// which `tracing::error!(message = %error)` does all over the launcher.
/// Both paths must render the same line — otherwise the console stays quiet
/// precisely where one is looking, quoted on one side and not on the other.
#[test]
fn a_message_passed_as_a_named_field_shows_like_the_others() {
    let output = console(|| {
        tracing::info!(message = "the pack is up to date");
    });

    assert!(
        output.contains("the pack is up to date"),
        "message missing: {output}"
    );
    // Rendered as a string, not through its `Debug` impl: that would
    // surround it with quotes.
    assert!(
        !output.contains("\"the pack is up to date\""),
        "message escaped: {output}"
    );
}
