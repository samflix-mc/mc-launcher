//! Track the log of a session in progress.

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::reading::{Crash, EXCERPT_LINES, split_exception, strip_ansi};

#[derive(Debug, Default)]
pub struct Watcher {
    /// Exception currently being accumulated and the number of lines already taken.
    pending: Option<(Crash, usize)>,
    found: Vec<Crash>,
    /// Avoids reporting the same one a hundred times: a mod that fails on
    /// every tick would fill the dashboard on its own.
    seen: std::collections::BTreeSet<String>,
}

/// Beyond this, reporting stops: a session that produces this many distinct
/// exceptions has a global problem, which the first ones already describe.
const MAX_DISTINCT: usize = 5;

impl Watcher {
    pub fn new() -> Watcher {
        Watcher::default()
    }

    /// Feeds a line of the game's output.
    pub fn line(&mut self, line: &str) {
        let clean = strip_ansi(line);

        // A trace continues through its frames; as long as they arrive,
        // they belong to the exception currently in progress.
        if let Some((crash, taken)) = &mut self.pending {
            let trimmed = clean.trim_start();
            let continues = trimmed.starts_with("at ")
                || trimmed.starts_with("Caused by:")
                || trimmed.starts_with("... ")
                || trimmed.starts_with("Suppressed:");
            if continues && *taken < EXCERPT_LINES {
                crash.excerpt.push('\n');
                crash.excerpt.push_str(&clean);
                *taken += 1;
                return;
            }
            let (finished, _) = self.pending.take().expect("present");
            self.keep(finished);
        }

        if self.found.len() >= MAX_DISTINCT {
            return;
        }
        if let Some((exception, message)) = split_exception(&clean) {
            self.pending = Some((
                Crash {
                    exception,
                    message,
                    excerpt: clean,
                    source: PathBuf::from("game output"),
                },
                0,
            ));
        }
    }

    fn keep(&mut self, crash: Crash) {
        let key = format!("{}: {}", crash.exception, crash.message);
        if self.seen.insert(key) {
            self.found.push(crash);
        }
    }

    /// Distinct exceptions noted during the run.
    pub fn finish(mut self) -> Vec<Crash> {
        if let Some((crash, _)) = self.pending.take() {
            self.keep(crash);
        }
        self.found
    }
}

/// List of loaded mods, to accompany a report.
///
/// A modpack crash almost always comes from a mod or a pair of mods; knowing
/// which ones were present saves a round trip.
pub fn loaded_mods(game_dir: &Path) -> Result<Vec<String>> {
    let mods = game_dir.join("mods");
    let Ok(entries) = std::fs::read_dir(&mods) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".jar"))
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
#[path = "watch.test.rs"]
mod tests;
