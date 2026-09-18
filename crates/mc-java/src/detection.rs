//! Retain an existing runtime, or install one.

use anyhow::Result;
use std::path::Path;

use crate::installation::install;
use crate::locations::{candidates, java_exe, managed_home};
use crate::version::{Java, Origin, probe};

pub async fn detect(major: u32, runtime_dir: &Path) -> Option<Java> {
    let managed = java_exe(&managed_home(runtime_dir, major));
    let mut seen = Vec::new();

    for exe in candidates(runtime_dir, major) {
        if !exe.is_file() {
            continue;
        }
        let real = tokio::fs::canonicalize(&exe)
            .await
            .unwrap_or_else(|_| exe.clone());
        if seen.contains(&real) {
            continue;
        }
        seen.push(real);

        // An executable can be present and broken (half-uninstalled package,
        // dead symlink): only what responds is retained.
        let Ok(version) = probe(&exe).await else {
            continue;
        };
        // `==` and not `>=`, and it's a requirement of the network, not a
        // preference. The lock names the major version NeoForge was installed
        // with; a newer Java changes mixin behavior and the registry format,
        // and the server cuts it off with an ejection that doesn't name its
        // cause. Accepting "at least" would let the player's machine choose
        // what the pack had pinned.
        //
        // The counterpart to this line is in `installation.rs`: without both,
        // `install` would accept what `detect` refuses, and `ensure` would
        // reinstall a hundred and eighty megabytes on every launch without
        // ever converging.
        if version.major == major {
            let origin = if exe == managed {
                Origin::Managed
            } else {
                Origin::System
            };
            return Some(Java {
                path: exe,
                version,
                origin,
            });
        }
    }
    None
}

/// Guarantees the presence of Java `major` EXACTLY: detection, otherwise
/// installation.
///
/// "Exactly" and not "at least", since the lock carries the major version
/// NeoForge was installed with. This is what the launcher checks on every
/// launch.
#[tracing::instrument(name = "runtime java", skip(runtime_dir, observer))]
pub async fn ensure(
    major: u32,
    runtime_dir: &Path,
    observer: Option<mc_dl::Observer>,
) -> Result<Java> {
    if let Some(java) = detect(major, runtime_dir).await {
        tracing::debug!(
            version = %java.version.full,
            path = %java.path.display(),
            "existing runtime retained"
        );
        return Ok(java);
    }
    // The only case that costs time and bandwidth: it deserves to be visible
    // without having to raise verbosity.
    tracing::info!(
        major = major,
        "No Java {major} on this machine, installing Temurin"
    );
    install(major, runtime_dir, observer).await
}

#[cfg(test)]
#[path = "detection.test.rs"]
mod tests;
