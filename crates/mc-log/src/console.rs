//! The console layer: what a user sees while waiting.

mod format;

use tracing_subscriber::{EnvFilter, Layer};

use crate::BoxedLayer;
use crate::file::Redacting;
use format::ConsoleFormat;

/// The console shows the essentials; the file keeps everything.
///
/// `RUST_LOG` tunes the former without touching the latter, so that a user
/// raising verbosity doesn't have to rerun the operation that failed.
pub(crate) fn layer() -> BoxedLayer {
    let filter = filter(std::env::var("RUST_LOG").ok().as_deref());

    tracing_subscriber::fmt::layer()
        .with_target(false)
        // Time elapsed since startup, not the absolute time. A command runs
        // for a few seconds; knowing a step took 4.2s is informative,
        // knowing it was 01:18:38 isn't.
        // The root span's fields would be repeated on every line —
        // "command{name=lock manifest=… environment=local}" seven times in
        // a row drowns out what one is trying to read. The file keeps them.
        .event_format(ConsoleFormat::new())
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(Redacting(std::io::stderr as fn() -> std::io::Stderr))
        .with_filter(filter)
        .boxed()
}

/// What `RUST_LOG` amounts to, once the two cases that would silently mute
/// the console are ruled out.
fn filter(raw: Option<&str>) -> EnvFilter {
    // A RUST_LOG that's set but empty is as good as one that's absent:
    // copying the example ".env" as-is sets it that way, and
    // `try_from_default_env` would then yield a filter with no directive at
    // all — a muted console, default included, with nothing to explain it.
    raw.filter(|level| !level.trim().is_empty())
        .and_then(|level| match EnvFilter::try_new(level) {
            Ok(filter) => Some(filter),
            // The subscriber isn't set up yet: this message can only go
            // through standard error. Silencing it would make a
            // malformed RUST_LOG indistinguishable from an absent one —
            // exactly the unexplained silence the previous case fixes.
            Err(error) => {
                eprintln!(
                    "RUST_LOG ignored ({error}): \"{level}\" — falling back to default filter."
                );
                None
            }
        })
        .unwrap_or_else(|| EnvFilter::new("info,hyper=warn,reqwest=warn,rustls=warn"))
}

#[cfg(test)]
#[path = "console.test.rs"]
mod tests;
