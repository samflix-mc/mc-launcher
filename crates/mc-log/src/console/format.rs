//! Formatting of a console line.

/// Console formatting: elapsed time, level, message.
///
/// `tracing-subscriber`'s default format repeats the current span's fields
/// in front of every line. That's valuable in a file reread later, unreadable
/// in a terminal: seven lines all preceded by the same
/// `command{name=lock manifest=… environment=local}` drown out what one is
/// trying to read.
///
/// Here the message carries the information and the fields come after,
/// unobtrusive. The file, on the other hand, keeps the full format with
/// spans.
pub(super) struct ConsoleFormat {
    /// Start of the run. Captured once: recreating the clock on every line
    /// would show zero everywhere, which was the first attempt.
    start: std::time::Instant,
}

impl ConsoleFormat {
    pub(super) fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for ConsoleFormat
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        use tracing_subscriber::fmt::FormatFields;

        // Two decimals are enough to place a step within a command that runs
        // for a few seconds; the default format's nanosecond only serves to
        // lengthen the line.
        write!(
            writer,
            "{:>6.2}s {:<5} ",
            self.start.elapsed().as_secs_f64(),
            event.metadata().level()
        )?;

        let level = *event.metadata().level();
        if level <= tracing::Level::INFO {
            // From `info` on, the message stands on its own — that's the
            // rule we've set ourselves. Repeating the fields it already
            // carries would double the line without teaching anything new.
            // They stay in the file and in Sentry, where they're used to
            // filter.
            let mut message = MessageOnly(String::new());
            event.record(&mut message);
            write!(writer, "{}", message.0)?;
        } else {
            // At `debug` and `trace`, the fields *are* the information: the
            // message is just a label above them.
            ctx.format_fields(writer.by_ref(), event)?;
        }
        writeln!(writer)
    }
}

/// Keeps only the `message` field of an event.
struct MessageOnly(String);

impl tracing::field::Visit for MessageOnly {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}

#[cfg(test)]
#[path = "format.test.rs"]
mod tests;
