//! What the command shows about the lock it just wrote.

use std::path::Path;

use mc_pack::lockfile::Lockfile;

use super::super::verify::report_unresolved;

/// Out of scope for mutation testing: this function has no effect other
/// than writing to standard output. What it shows is checked — that's
/// [`lines`], right below.
#[mutants::skip]
pub(super) fn announce(lock: &Lockfile, lock_path: &Path, previous: Option<&Lockfile>) {
    for line in lines(lock, lock_path, previous) {
        println!("{line}");
    }
    report_unresolved(lock);
}

/// The report, separate from how it's shown.
///
/// It's what you reread to know what a resolution changed: the loader, the
/// number of mods, each with its side and who requested it, then the
/// differences from the previous lock. Staying silent on the latter would
/// suggest a resolution changed nothing when it replaced ten builds.
pub(super) fn lines(lock: &Lockfile, lock_path: &Path, previous: Option<&Lockfile>) -> Vec<String> {
    let mut lines = vec![format!(
        "NeoForge {} — {} mods",
        lock.loader.version,
        lock.mods.len()
    )];

    lines.extend(lock.mods.iter().map(|entry| {
        format!(
            "  {:<24} {:<40} {:<7} {}",
            entry.slug, entry.file_name, entry.side, entry.reason
        )
    }));

    if let Some(previous) = previous {
        let changes = lock.diff(previous);
        if !changes.is_empty() {
            lines.push("\nChanges:".to_string());
            lines.extend(changes.into_iter().map(|line| format!("  {line}")));
        }
    }

    lines.push(format!("\nLock written to {}", lock_path.display()));
    lines
}

#[cfg(test)]
#[path = "report.test.rs"]
mod tests;
