//! Poser un client ou un serveur NeoForge à partir de l'installateur.

use anyhow::{bail, Result};
use mc_dl::Downloader;
use std::path::{Path, PathBuf};

use super::execution::run_installer;
use super::{fetch_installer, version_id};

#[tracing::instrument(name = "neoforge client", skip(shared, cache, java, dl))]
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
        tracing::debug!(version, "déjà installé, installateur non relancé");
        return Ok(produced);
    }

    let installer = fetch_installer(version, cache, dl).await?;
    // L'installateur applique des patchs binaires : sur une machine lente c'est
    // une minute pendant laquelle rien ne bouge à l'écran.
    tracing::info!(
        version,
        "Exécution de l'installateur NeoForge {version} — peut prendre une minute"
    );

    // L'installateur refuse de démarrer sans ce fichier : il y inscrit un
    // profil pour le launcher officiel. On n'en fait rien, mais son absence
    // est une erreur fatale de son point de vue.
    let profiles = shared.join("launcher_profiles.json");
    if !profiles.is_file() {
        tokio::fs::create_dir_all(shared).await?;
        tokio::fs::write(&profiles, br#"{"profiles":{},"version":3}"#).await?;
    }

    run_installer(&installer, "--install-client", shared, java).await?;

    if !produced.is_file() {
        bail!(
            "l'installateur NeoForge {version} s'est terminé sans produire {}",
            produced.display()
        );
    }
    Ok(produced)
}

/// Installe un serveur NeoForge complet dans son propre répertoire.
#[tracing::instrument(name = "neoforge serveur", skip(dir, cache, java, dl))]
pub async fn install_server(
    version: &str,
    dir: &Path,
    cache: &Path,
    java: &Path,
    dl: &Downloader,
) -> Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    // L'installateur va dans le cache partagé, pas dans le répertoire du
    // serveur : celui-ci est destiné à être recopié vers l'hôte qui fait
    // tourner le jeu, et n'a pas à emporter six mégaoctets d'outillage.
    let installer = fetch_installer(version, cache, dl).await?;
    run_installer(&installer, "--install-server", dir, java).await?;
    Ok(())
}
