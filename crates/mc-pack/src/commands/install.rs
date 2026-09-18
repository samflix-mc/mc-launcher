//! Install the pack, then say what was placed.

use std::sync::Arc;

use anyhow::Result;
use mc_pack::source::Source;

use super::verify::report_unresolved;

/// A terminal's report: lines, in the order they arrive.
///
/// Steps aren't printed — the notes already announce them, in English and
/// with their numbers. They serve a window, which needs to know *which one*
/// is working in order to draw a path; a terminal just stacks them.
///
/// Downloads aren't printed either: a rate that changes ten times a second
/// scrolls faster than it reads, and would drown out the report.
struct Terminal;

impl mc_pack::Report for Terminal {
    fn step(&self, _step: mc_pack::Step) {}

    /// Out of scope for mutation testing: it does nothing but write to
    /// standard output, which Rust has no stable way to read back from the
    /// process that emits it. The TEXT, though, is computed elsewhere — by
    /// `Tracker` and by the install reports — and tested where it's computed.
    #[mutants::skip]
    fn note(&self, text: &str) {
        println!("{text}");
    }
}

/// Out of scope for mutation testing: this function installs the pack for
/// real — it downloads Minecraft, NeoForge, and a hundred mods — then shows
/// the result. What it shows is tested; what it installs is tested by
/// `mc_pack::install`'s own suite, which has a test server.
#[mutants::skip]
pub async fn install(source: &Source, options: &mc_pack::Options) -> Result<()> {
    let outcome = mc_pack::install(source, options, Arc::new(Terminal)).await?;

    for line in lines(&outcome) {
        println!("{line}");
    }
    report_unresolved(&outcome.lock);
    Ok(())
}

/// The report of an installation, separate from how it's shown.
///
/// It's what a player rereads to know where the game was placed, and what an
/// update changed. Two lists only appear if they have something to say:
/// removed mods, and differences from the previous lock. A title followed by
/// nothing would send someone looking for a change that never happened.
pub(super) fn lines(outcome: &mc_pack::Outcome) -> Vec<String> {
    let mut lines = vec![
        format!("\nInstance \"{}\"", outcome.instance.name),
        format!("  pack     : {}", outcome.source),
    ];
    if outcome.from_cache {
        lines.push("             (offline — local copy, not the published pack)".to_string());
    }
    lines.push(format!(
        "  game     : {}",
        outcome.instance.game_dir.display()
    ));
    lines.push(format!(
        "  mods     : {} client-side, {} server-side",
        outcome.client_mods, outcome.server_mods
    ));
    lines.push(format!("  server   : {}", outcome.server_dir.display()));
    lines.push(format!("  lock     : {}", outcome.lock_path.display()));

    if !outcome.removed.is_empty() {
        lines.push(format!("  removed  : {}", outcome.removed.join(", ")));
    }
    if let Some(previous) = &outcome.previous_lock {
        let changes = outcome.lock.diff(previous);
        if !changes.is_empty() {
            lines.push("\nChanges since the previous lock:".to_string());
            lines.extend(changes.into_iter().map(|line| format!("  {line}")));
        }
    }
    lines
}

#[cfg(test)]
#[path = "install.test.rs"]
mod tests;
