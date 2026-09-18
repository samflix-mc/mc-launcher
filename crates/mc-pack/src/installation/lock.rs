//! Write the lock, or obey the one being replayed.

use std::path::Path;

use anyhow::Result;

use crate::lockfile::{LockedLoader, Lockfile};
use crate::manifest::Manifest;

#[allow(clippy::too_many_arguments)]
pub(super) fn retain(
    manifest: &Manifest,
    neoforge_version: &str,
    java_major: u32,
    plan: &mc_mods::Plan,
    previous: Option<&Lockfile>,
    lock_path: &Path,
    replay: bool,
) -> Result<Lockfile> {
    //
    // Replaying a lock means obeying it, not rewriting it. Regenerating it
    // would wipe the `reason` column: everything in it would become
    // "requested by the manifest", since it's the lock itself that dictated
    // the requests, and the only trace of what had never been requested
    // would be lost.
    let lock = match (replay, previous) {
        (true, Some(existing)) => {
            tracing::debug!(
                lock = %lock_path.display(),
                "lock replayed, left as is"
            );
            existing.clone()
        }
        _ => {
            let fresh = Lockfile::from_plan(
                manifest,
                LockedLoader {
                    kind: manifest.loader.kind.clone(),
                    version: neoforge_version.to_string(),
                },
                java_major,
                plan,
            );
            fresh.save(lock_path)?;
            fresh
        }
    };
    Ok(lock)
}

#[cfg(test)]
#[path = "lock.test.rs"]
mod tests;
