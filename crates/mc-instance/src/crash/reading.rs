//! What a crash is, and how it's extracted from text.

mod parsing;

use std::path::PathBuf;

pub(super) use parsing::{split_exception, strip_ansi};

/// What could be learned from an abnormal stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crash {
    /// Type of the Java exception, e.g. `java.lang.module.ResolutionException`.
    ///
    /// It serves as the title: without it, all of the game's crashes would
    /// merge into a single indistinct incident.
    pub exception: String,
    /// Message carried by the exception.
    pub message: String,
    /// Excerpt from the file, bounded to stay readable and sendable.
    pub excerpt: String,
    /// File the information comes from.
    pub source: PathBuf,
}

/// Number of lines kept around the exception.
///
/// Enough for the trace and the immediate context, not enough to exceed
/// the limits of a Sentry event nor to become unreadable.
pub(super) const EXCERPT_LINES: usize = 60;

/// (`Caused by`, closing exceptions), and it's the original one that
/// identifies the problem.
pub fn parse(text: &str) -> Option<Crash> {
    let lines: Vec<&str> = text.lines().collect();

    let (index, exception, message) = lines.iter().enumerate().find_map(|(i, line)| {
        let (exception, message) = split_exception(line)?;
        Some((i, exception, message))
    })?;

    // The excerpt starts a bit earlier: the lines before it often say what
    // the game was doing.
    let start = index.saturating_sub(5);
    let end = (index + EXCERPT_LINES).min(lines.len());
    let excerpt = lines[start..end].join("\n");

    Some(Crash {
        exception,
        message,
        excerpt: strip_ansi(&excerpt),
        source: PathBuf::new(),
    })
}

#[cfg(test)]
#[path = "reading.test.rs"]
mod tests;
