//! Récupérer l'installateur officiel de NeoForge.

mod execution;

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum};
use mc_dl::Downloader;
use std::path::{Path, PathBuf};

use super::MAVEN;
mod pose;

pub use pose::{install_client, install_server};


fn installer_url(version: &str) -> String {
    format!("{MAVEN}/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar")
}

/// Télécharge l'installateur et vérifie son empreinte.
///
/// Le `.sha1` publié à côté de l'artefact est l'empreinte de référence du
/// dépôt. L'installateur est exécuté juste après : le vérifier n'est pas une
/// formalité.
async fn fetch_installer(version: &str, cache: &Path, dl: &Downloader) -> Result<PathBuf> {
    let url = installer_url(version);
    let dest = cache.join(format!("neoforge-{version}-installer.jar"));

    let sha1 = dl
        .bytes(&format!("{url}.sha1"))
        .await
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
        .with_context(|| format!("empreinte de l'installateur NeoForge {version}"))?;

    dl.to_file(&url, &dest, Check::Full(&Checksum::Sha1(sha1)))
        .await
        .with_context(|| format!("téléchargement de l'installateur NeoForge {version}"))?;
    Ok(dest)
}

/// Nom du répertoire de version produit par l'installateur.
pub fn version_id(version: &str) -> String {
    format!("neoforge-{version}")
}

#[cfg(test)]
#[path = "installateur.test.rs"]
mod tests;
