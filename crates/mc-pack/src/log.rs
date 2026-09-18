//! What gets written to the log when a command finishes.

use std::process::ExitCode;
use std::time::Instant;

use anyhow::Result;

use mc_pack::source::Source;

/// Root span: everything that follows is attached to it, and its duration is
/// the command's. In the file just as in Sentry, a run reads as one block
/// this way even when several follow each other.
pub fn open(command: &str, source: &Source) -> tracing::span::EnteredSpan {
    let span = tracing::info_span!(
        "command",
        name = %command,
        pack = %source.describe(),
        environment = mc_log::environment::current().as_str(),
    );
    let entered = span.entered();
    tracing::info!(
        environment = mc_log::environment::current().as_str(),
        "mc-pack {command} on {} — environment {}",
        source.describe(),
        mc_log::environment::current().as_str()
    );
    entered
}

/// An error that surfaces this far ends the program: it's the last place it
/// can become an incident rather than a plain message.
///
/// Out of reach of mutation tests: all this function does is emit a log line
/// — which the Sentry layer turns into an incident, which is checked on its
/// own side — and write the detailed file's path to standard error. Rust
/// lets us read back neither one from the process that produces them.
#[mutants::skip]
pub fn conclude(command: &str, result: &Result<ExitCode>, start: Instant, log: &mc_log::Guard) {
    match &result {
        Ok(_) => tracing::info!(
            duration_ms = start.elapsed().as_millis(),
            "mc-pack {command} finished in {:.1}s",
            start.elapsed().as_secs_f64()
        ),
        Err(error) => {
            tracing::error!(
                duration_ms = start.elapsed().as_millis(),
                error = ?error,
                "mc-pack {command} failed after {:.1}s: {error}",
                start.elapsed().as_secs_f64()
            );
            if let Some(path) = log.log_path() {
                eprintln!("\nDetailed log: {}", path.display());
            }
        }
    }
}

#[cfg(test)]
#[path = "log.test.rs"]
mod tests;
