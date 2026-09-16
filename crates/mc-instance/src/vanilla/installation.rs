//! Installer une version : le descripteur, le client, et ce qu'ils entraînent.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::{Path, PathBuf};

use super::assets::install_assets;
use super::bibliotheques::install_libraries;
use super::descripteur::{Manifest, VersionJson};
use super::MANIFEST;

/// Ce que l'installation vanilla a produit.
#[derive(Debug)]
pub struct Vanilla {
    pub id: String,
    pub main_class: String,
    /// Version majeure de Java exigée par Mojang pour cette version du jeu.
    pub java_major: u32,
    pub client_jar: PathBuf,
    pub version_json: PathBuf,
    pub libraries: Vec<PathBuf>,
    pub asset_index_id: String,
    pub assets_downloaded: usize,
}

/// Installe les fichiers Mojang d'une version dans un répertoire partagé.
#[tracing::instrument(name = "jeu vanilla", skip(shared, dl))]
pub async fn install(mc: &str, shared: &Path, dl: &Downloader) -> Result<Vanilla> {
    let manifest: Manifest = serde_json::from_slice(&dl.bytes(MANIFEST).await?)
        .context("manifeste des versions illisible")?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == mc)
        .with_context(|| format!("Minecraft {mc} ne figure pas au manifeste de Mojang"))?;

    // Le descripteur est vérifié comme le reste : son SHA-1 figure dans le
    // manifeste, et c'est lui qui donne les empreintes de tous les autres
    // fichiers. Le corrompre reviendrait à corrompre l'installation entière.
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
    .context("téléchargement du descripteur de version")?;

    let version: VersionJson = serde_json::from_slice(&tokio::fs::read(&version_json).await?)
        .with_context(|| format!("{} illisible", version_json.display()))?;

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
    .context("téléchargement du client")?;

    tracing::debug!(
        version = %version.id,
        java_exige = version.java_version.as_ref().map(|j| j.major_version),
        bibliotheques_declarees = version.libraries.len(),
        "descripteur de version lu"
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
