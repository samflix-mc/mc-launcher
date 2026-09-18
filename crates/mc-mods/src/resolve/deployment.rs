//! From the cache to the instance's `mods` folder.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

use crate::jar::Side;

use super::plan::Plan;

/// would stay loaded and would drift the server's registry.
pub fn deploy(plan: &Plan, side: Side, mods_dir: &Path) -> Result<Deployed> {
    std::fs::create_dir_all(mods_dir)
        .with_context(|| format!("creating {}", mods_dir.display()))?;

    let wanted: BTreeSet<String> = plan
        .for_side(side)
        .map(|m| m.candidate.file_name.clone())
        .collect();

    let mut removed = Vec::new();
    for entry in std::fs::read_dir(mods_dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jar") || wanted.contains(&name) {
            continue;
        }
        std::fs::remove_file(entry.path())?;
        removed.push(name);
    }

    let mut installed = 0;
    for entry in plan.for_side(side) {
        let dest = mods_dir.join(&entry.candidate.file_name);
        if dest.exists() {
            let same = match entry.candidate.checksum() {
                Some(expected) => std::fs::read(&dest)
                    .map(|bytes| expected.matches(&bytes))
                    .unwrap_or(false),
                None => true,
            };
            if same {
                installed += 1;
                continue;
            }
            std::fs::remove_file(&dest)?;
        }
        link_or_copy(&entry.path, &dest)?;
        installed += 1;
    }

    Ok(Deployed { installed, removed })
}

#[derive(Debug)]
pub struct Deployed {
    pub installed: usize,
    pub removed: Vec<String>,
}

/// Hard link if possible, copy otherwise.
///
/// A pack weighs several hundred megabytes and the same jar often serves
/// both the client and the server: the link avoids storing it three times.
/// It fails across different filesystems, hence the fallback.
fn link_or_copy(from: &Path, to: &Path) -> Result<()> {
    if std::fs::hard_link(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)
        .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "deployment.test.rs"]
mod tests;
