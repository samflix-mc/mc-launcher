//! Verify that an installation is complete and intact.

use anyhow::Result;
use mc_pack::lockfile::Lockfile;
use mc_pack::source::Source;
use std::process::ExitCode;

pub fn verify(source: &Source, options: &mc_pack::Options, deep: bool) -> Result<ExitCode> {
    let problems = mc_pack::verify(source, options, deep)?;
    if problems.is_empty() {
        tracing::info!(
            thorough = deep,
            "Installation matches the lock{}",
            if deep { ", digests included" } else { "" }
        );
        println!("Installation complete and matching the lock.");
        return Ok(ExitCode::SUCCESS);
    }
    // A verification mismatch isn't a program failure: it describes the
    // installation. Hence `warn` rather than `error` — the incident is the
    // exit code, which the caller already sees.
    tracing::warn!(
        anomalies = problems.len(),
        thorough = deep,
        "Installation does not match the lock: {} anomalies",
        problems.len()
    );
    for problem in &problems {
        tracing::debug!(anomaly = %problem, "detail");
    }
    eprintln!("{} anomalies:", problems.len());
    for problem in &problems {
        eprintln!("  {problem}");
    }
    Ok(ExitCode::FAILURE)
}

/// A missing dependency doesn't stop the install, but it will stop the game
/// from starting: it's flagged where it'll be seen.
///
/// Out of scope for mutation testing: this function only writes to standard
/// error. What it says is checked — that's [`unresolved_lines`].
#[mutants::skip]
pub(super) fn report_unresolved(lock: &Lockfile) {
    for line in unresolved_lines(lock) {
        eprintln!("{line}");
    }
}

/// The report of missing dependencies, separate from how it's shown.
///
/// Empty when there's nothing to say: a title with no list would send
/// someone looking for a problem that isn't there. Otherwise, each missing
/// piece is named along with who required it — it's the first place to
/// look when the game refuses to start.
pub(super) fn unresolved_lines(lock: &Lockfile) -> Vec<String> {
    if lock.unresolved.is_empty() {
        return Vec::new();
    }
    let mut lines = vec!["\nMissing dependencies:".to_string()];
    lines.extend(lock.unresolved.iter().map(|missing| {
        format!(
            "  {} — required by {} ({})",
            missing.mod_id, missing.required_by, missing.side
        )
    }));
    lines.push(
        "  These mods are missing from both Modrinth and CurseForge: the game will refuse to start."
            .to_string(),
    );
    lines
}

#[cfg(test)]
#[path = "verify.test.rs"]
mod tests;
