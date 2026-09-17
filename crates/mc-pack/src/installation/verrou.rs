//! Écrire le verrou, ou obéir à celui qu'on rejoue.

use std::path::Path;

use anyhow::Result;

use crate::lockfile::{LockedLoader, Lockfile};
use crate::manifest::Manifest;

#[allow(clippy::too_many_arguments)]
pub(super) fn retenir(
    manifest: &Manifest,
    neoforge_version: &str,
    java_major: u32,
    plan: &mc_mods::Plan,
    previous: Option<&Lockfile>,
    lock_path: &Path,
    replay: bool,
) -> Result<Lockfile> {
    //
    // Rejouer un verrou, c'est lui obéir, pas le réécrire. Le régénérer
    // effacerait la colonne `reason` : tout y deviendrait « demandé par le
    // manifeste », puisque c'est le verrou lui-même qui a dicté les demandes,
    // et on perdrait la seule trace de ce qui n'avait jamais été demandé.
    let lock = match (replay, previous) {
        (true, Some(existing)) => {
            tracing::debug!(
                verrou = %lock_path.display(),
                "verrou rejoué, laissé tel quel"
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
#[path = "verrou.test.rs"]
mod tests;
