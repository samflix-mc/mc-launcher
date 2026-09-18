//! Fetch NeoForge's official installer.

mod execution;

use anyhow::{Context, Result};
use mc_dl::Downloader;
use mc_dl::{Check, Checksum};
use std::path::{Path, PathBuf};

use super::MAVEN;
mod placement;

pub use placement::{install_client, install_server};

fn installer_url(version: &str) -> String {
    format!("{MAVEN}/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar")
}

/// Downloads the installer and verifies its digest.
///
/// The `.sha1` published alongside the artifact is the repository's
/// reference digest. The installer runs right after: verifying it isn't a
/// formality.
/// Out of scope for mutation testing: this function only downloads a jar
/// from maven.neoforged.net, at an address written into this module. The
/// download itself — resume, digest, atomic write — is verified in mc-dl,
/// which has a test server.
#[mutants::skip]
async fn fetch_installer(version: &str, cache: &Path, dl: &Downloader) -> Result<PathBuf> {
    let url = installer_url(version);
    let dest = cache.join(format!("neoforge-{version}-installer.jar"));

    let sha1 = dl
        .bytes(&format!("{url}.sha1"))
        .await
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
        .with_context(|| format!("digest of the NeoForge {version} installer"))?;

    dl.to_file(&url, &dest, Check::Full(&Checksum::Sha1(sha1)))
        .await
        .with_context(|| format!("downloading the NeoForge {version} installer"))?;
    Ok(dest)
}

/// Name of the version directory the installer produces.
pub fn version_id(version: &str) -> String {
    format!("neoforge-{version}")
}

#[cfg(test)]
#[path = "installer.test.rs"]
mod tests;
