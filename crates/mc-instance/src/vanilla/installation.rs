//! Installer une version : le descripteur, le client, et ce qu'ils entraînent.

use anyhow::{Context, Result};
use mc_dl::{Check, Checksum, Downloader};
use std::path::{Path, PathBuf};

use super::MANIFEST;
use super::assets::install_assets;
use super::bibliotheques::install_libraries;
use super::descripteur::{Manifest, VersionJson};

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
    let entry = entree_de_version(manifest.versions, mc)
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

/// Ce que MOJANG exige comme majeure de Java pour cette version du jeu.
///
/// ## Pourquoi cette fonction existe
///
/// `mc-pack lock` écrivait `unwrap_or(21)` : une constante, dans le fichier
/// même qui prétend figer ce qui sera installé. L'installation, elle,
/// interrogeait Mojang. Les deux pouvaient donc diverger — et le verrou, qui
/// est censé être la vérité, portait le chiffre deviné.
///
/// Cette fonction est ce que l'installation lit, extrait pour que `lock`
/// puisse lire la même chose. Elle ne télécharge que deux JSON — le manifeste
/// des versions et le descripteur — là où une installation complète descend
/// plusieurs centaines de mégaoctets.
///
/// ## Pourquoi `Option` et non `u32`
///
/// Les descripteurs anciens n'ont pas de bloc `javaVersion` : Mojang ne l'a
/// ajouté qu'avec la 1.17. Rendre 21 à leur place serait réinventer le
/// `unwrap_or` qu'on retire. C'est à l'appelant de décider, et il le fait
/// déjà : `Manifest::java_major` donne la priorité à ce que le manifeste du
/// pack déclare, et ne retombe sur celui-ci qu'à défaut.
#[tracing::instrument(name = "java exigé", skip(dl))]
pub async fn java_exige(mc: &str, dl: &Downloader) -> Result<Option<u32>> {
    let manifest: Manifest = serde_json::from_slice(&dl.bytes(MANIFEST).await?)
        .context("manifeste des versions illisible")?;
    let entry = entree_de_version(manifest.versions, mc)
        .with_context(|| format!("Minecraft {mc} ne figure pas au manifeste de Mojang"))?;

    let version: VersionJson = serde_json::from_slice(&dl.bytes(&entry.url).await?)
        .with_context(|| format!("descripteur de {mc} illisible"))?;

    Ok(version.java_version.map(|j| j.major_version))
}

/// L'entrée du manifeste Mojang qui décrit exactement cette version du jeu.
///
/// Le manifeste en énumère plusieurs centaines, des instantanés compris. Se
/// tromper d'entrée installerait un autre jeu que celui demandé, avec ses
/// bibliothèques et ses assets — et le pack ne démarrerait pas, pour une
/// raison qui ne se lirait nulle part.
fn entree_de_version(
    versions: Vec<super::descripteur::ManifestVersion>,
    mc: &str,
) -> Option<super::descripteur::ManifestVersion> {
    versions.into_iter().find(|v| v.id == mc)
}

#[cfg(test)]
#[path = "installation.test.rs"]
mod tests;
