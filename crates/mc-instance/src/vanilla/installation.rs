//! Install a version: the descriptor, the client, and what they pull in.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::{Path, PathBuf};

use super::MANIFEST;
use super::assets::install_assets;
use super::descriptor::{Manifest, VersionJson};
use super::libraries::install_libraries;

/// What the vanilla installation produced.
#[derive(Debug)]
pub struct Vanilla {
    pub id: String,
    pub main_class: String,
    /// Major Java version required by Mojang for this game version.
    pub java_major: u32,
    pub client_jar: PathBuf,
    pub version_json: PathBuf,
    pub libraries: Vec<PathBuf>,
    pub asset_index_id: String,
    pub assets_downloaded: usize,
}

/// Installs a version's Mojang files into a shared directory.
#[tracing::instrument(name = "vanilla game", skip(shared, dl))]
pub async fn install(mc: &str, shared: &Path, dl: &Downloader) -> Result<Vanilla> {
    let manifest: Manifest = serde_json::from_slice(&dl.bytes(MANIFEST).await?)
        .context("unreadable version manifest")?;
    let entry = version_entry(manifest.versions, mc)
        .with_context(|| format!("Minecraft {mc} is not in Mojang's manifest"))?;

    // The descriptor is verified like everything else: its SHA-1 is in the
    // manifest, and it's the one that gives the digests of every other file.
    // Corrupting it would mean corrupting the whole installation.
    let version_json = shared
        .join("versions")
        .join(&entry.id)
        .join(format!("{}.json", entry.id));
    dl.to_file(
        &entry.url,
        &version_json,
        Check::Full(&Checksum::Sha1(entry.sha1.clone())),
    )
    .await
    .context("downloading the version descriptor")?;

    let version: VersionJson = serde_json::from_slice(&tokio::fs::read(&version_json).await?)
        .with_context(|| format!("{} unreadable", version_json.display()))?;

    let client_jar = shared
        .join("versions")
        .join(&version.id)
        .join(format!("{}.jar", version.id));
    dl.to_file(
        &version.downloads.client.url,
        &client_jar,
        Check::Full(&Checksum::Sha1(version.downloads.client.sha1.clone())),
    )
    .await
    .context("downloading the client")?;

    tracing::debug!(
        version = %version.id,
        java_required = version.java_version.as_ref().map(|j| j.major_version),
        declared_libraries = version.libraries.len(),
        "version descriptor read"
    );

    let libraries = install_libraries(&version, shared, dl).await?;
    let assets_downloaded = install_assets(&version.asset_index, shared, dl).await?;

    Ok(Vanilla {
        java_major: version.java_version.map(|j| j.major_version).unwrap_or(21),
        id: version.id,
        main_class: version.main_class,
        client_jar,
        version_json,
        libraries,
        asset_index_id: version.asset_index.id,
        assets_downloaded,
    })
}

/// What MOJANG requires as the Java major version for this game version.
///
/// ## Why this function exists
///
/// `mc-pack lock` used to write `unwrap_or(21)`: a constant, in the very file
/// that claims to pin what will be installed. The installer, meanwhile,
/// queried Mojang. The two could therefore diverge — and the lock, which is
/// supposed to be the source of truth, carried the guessed figure.
///
/// This function is what the installer reads, extracted so that `lock` can
/// read the same thing. It only downloads two JSON files — the version
/// manifest and the descriptor — where a full installation pulls down several
/// hundred megabytes.
///
/// ## Why `Option` and not `u32`
///
/// Older descriptors have no `javaVersion` block: Mojang only added it with
/// 1.17. Returning 21 in its place would just reinvent the `unwrap_or` we're
/// removing. It's up to the caller to decide, and it already does:
/// `Manifest::java_major` prioritizes what the pack manifest declares, and
/// only falls back to this one when it doesn't.
#[tracing::instrument(name = "java required", skip(dl))]
/// Out of scope for mutation testing: the two JSON files it reads come from
/// `launchermeta.mojang.com`, at an address written into this module. We
/// don't redirect it to a test server, and faking Mojang's manifest would
/// amount to verifying our own imitation of it.
///
/// What it DECIDES, on the other hand, is proven elsewhere and that's what
/// matters: `Manifest::java_major` prioritizes the pack manifest and only
/// falls back to this value otherwise — `manifest/reading.requests.test.rs`
/// locks that in — and the strict equality of the major version is proven on
/// both sides in `mc-java`.
#[mutants::skip]
pub async fn java_required(mc: &str, dl: &Downloader) -> Result<Option<u32>> {
    let manifest: Manifest = serde_json::from_slice(&dl.bytes(MANIFEST).await?)
        .context("unreadable version manifest")?;
    let entry = version_entry(manifest.versions, mc)
        .with_context(|| format!("Minecraft {mc} is not in Mojang's manifest"))?;

    let version: VersionJson = serde_json::from_slice(&dl.bytes(&entry.url).await?)
        .with_context(|| format!("descriptor for {mc} unreadable"))?;

    Ok(version.java_version.map(|j| j.major_version))
}

/// The entry in Mojang's manifest that describes exactly this game version.
///
/// The manifest lists several hundred of them, snapshots included. Picking
/// the wrong entry would install a different game than the one requested,
/// with its own libraries and assets — and the pack wouldn't start, for a
/// reason that wouldn't show up anywhere.
fn version_entry(
    versions: Vec<super::descriptor::ManifestVersion>,
    mc: &str,
) -> Option<super::descriptor::ManifestVersion> {
    versions.into_iter().find(|v| v.id == mc)
}

#[cfg(test)]
#[path = "installation.test.rs"]
mod tests;
