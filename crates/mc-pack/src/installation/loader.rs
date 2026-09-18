//! Which version of NeoForge, and how it is placed.

use std::path::Path;

use anyhow::{Context, Result};

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

/// The version to place: the one from the lock when it is replayed,
/// otherwise the one the manifest pins, otherwise the latest published.
pub(super) async fn version(
    manifest: &Manifest,
    previous: Option<&Lockfile>,
    lock_path: &Path,
    replay: bool,
    dl: &mc_dl::Downloader,
) -> Result<String> {
    let neoforge_version = if replay {
        let lock = previous
            .with_context(|| format!("{} missing: nothing to replay", lock_path.display()))?;
        lock.loader.version.clone()
    } else if manifest.loader.is_latest() {
        mc_instance::neoforge::latest_for(&manifest.minecraft, dl).await?
    } else {
        manifest.loader.version.clone()
    };
    tracing::info!(
        minecraft = %manifest.minecraft,
        neoforge = %neoforge_version,
        pinned = !manifest.loader.is_latest(),
        "Minecraft {} with NeoForge {neoforge_version}",
        manifest.minecraft
    );
    Ok(neoforge_version)
}

/// Places the loader: it writes its own `version.json`, from which the
/// command line is later derived.
/// Out of scope for mutation testing: this function downloads the NeoForge
/// installer and runs it in a JVM. What it delegates is verified in
/// `mc_instance::neoforge`, which knows how to do it with a test server and
/// a fake java.
#[mutants::skip]
pub(super) async fn place(
    version: &str,
    shared: &Path,
    layout: &mc_instance::Layout,
    java: &Path,
    dl: &mc_dl::Downloader,
) -> Result<()> {
    mc_instance::neoforge::install_client(version, shared, &layout.cache(), java, dl)
        .await
        .with_context(|| format!("installing NeoForge {version}"))?;
    tracing::info!(version, "NeoForge loader {version} in place");
    Ok(())
}

#[cfg(test)]
#[path = "loader.test.rs"]
mod tests;
