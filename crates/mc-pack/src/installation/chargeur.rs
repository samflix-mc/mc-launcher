//! Quelle version de NeoForge, et comment on la pose.

use std::path::Path;

use anyhow::{Context, Result};

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

/// La version à poser : celle du verrou quand on le rejoue, sinon celle que le
/// manifeste épingle, sinon la dernière publiée.
pub(super) async fn version(
    manifest: &Manifest,
    previous: Option<&Lockfile>,
    lock_path: &Path,
    replay: bool,
    dl: &mc_dl::Downloader,
) -> Result<String> {
    let neoforge_version = if replay {
        let lock = previous
            .with_context(|| format!("{} absent : rien à rejouer", lock_path.display()))?;
        lock.loader.version.clone()
    } else if manifest.loader.is_latest() {
        mc_instance::neoforge::latest_for(&manifest.minecraft, dl).await?
    } else {
        manifest.loader.version.clone()
    };
    tracing::info!(
        minecraft = %manifest.minecraft,
        neoforge = %neoforge_version,
        epingle = !manifest.loader.is_latest(),
        "Minecraft {} avec NeoForge {neoforge_version}",
        manifest.minecraft
    );
    Ok(neoforge_version)
}

/// Pose le chargeur : c'est lui qui écrit son propre `version.json`, dont la
/// ligne de commande dérive ensuite.
pub(super) async fn poser(
    version: &str,
    shared: &Path,
    layout: &mc_instance::Layout,
    java: &Path,
    dl: &mc_dl::Downloader,
) -> Result<()> {
    mc_instance::neoforge::install_client(version, shared, &layout.cache(), java, dl)
        .await
        .with_context(|| format!("installation de NeoForge {version}"))?;
    tracing::info!(version, "Chargeur NeoForge {version} en place");
    Ok(())
}
