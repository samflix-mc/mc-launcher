//! Place a NeoForge client or server from the installer.

use anyhow::{Result, bail};
use mc_dl::Downloader;
use std::path::{Path, PathBuf};

use super::execution::run_installer;
use super::{fetch_installer, version_id};

#[tracing::instrument(name = "neoforge client", skip(shared, cache, java, dl))]
/// Out of scope for mutation testing: this function downloads NeoForge's
/// official installer and runs it in a JVM, for about a minute. What it
/// checks afterward — that the expected profile was actually produced —
/// can only be told apart by running that very installer; faking it would
/// amount to verifying our own imitation.
#[mutants::skip]
pub async fn install_client(
    version: &str,
    shared: &Path,
    cache: &Path,
    java: &Path,
    dl: &Downloader,
) -> Result<PathBuf> {
    let produced = shared
        .join("versions")
        .join(version_id(version))
        .join(format!("{}.json", version_id(version)));
    if produced.is_file() {
        tracing::debug!(version, "already installed, installer not rerun");
        return Ok(produced);
    }

    let installer = fetch_installer(version, cache, dl).await?;
    // The installer applies binary patches: on a slow machine that's a
    // minute during which nothing moves on screen.
    tracing::info!(
        version,
        "Running the NeoForge {version} installer — may take a minute"
    );

    // The installer refuses to start without this file: it writes a profile
    // for the official launcher into it. We do nothing with it, but its
    // absence is a fatal error from the installer's point of view.
    let profiles = shared.join("launcher_profiles.json");
    if !profiles.is_file() {
        tokio::fs::create_dir_all(shared).await?;
        tokio::fs::write(&profiles, br#"{"profiles":{},"version":3}"#).await?;
    }

    run_installer(&installer, "--install-client", shared, java).await?;

    if !produced.is_file() {
        bail!(
            "the NeoForge {version} installer finished without producing {}",
            produced.display()
        );
    }
    Ok(produced)
}

/// Installs a full NeoForge server into its own directory.
#[tracing::instrument(name = "neoforge server", skip(dir, cache, java, dl))]
/// Out of scope for mutation testing, for the same reason as
/// [`install_client`]: it's NeoForge's installer doing the work.
#[mutants::skip]
pub async fn install_server(
    version: &str,
    dir: &Path,
    cache: &Path,
    java: &Path,
    dl: &Downloader,
) -> Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    // The installer goes into the shared cache, not into the server
    // directory: the latter is meant to be copied onto the host running the
    // game, and shouldn't carry six megabytes of tooling.
    let installer = fetch_installer(version, cache, dl).await?;
    run_installer(&installer, "--install-server", dir, java).await?;
    Ok(())
}
