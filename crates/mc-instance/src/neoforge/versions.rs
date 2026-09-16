//! Quelle version de NeoForge pour quelle version de Minecraft.

use anyhow::{Context, Result};
use mc_dl::Downloader;
use serde::Deserialize;

const VERSIONS_API: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
struct VersionList {
    versions: Vec<String>,
}

/// Série NeoForge correspondant à une version de Minecraft.
///
/// NeoForge numérote `<majeur>.<mineur>.<correctif>` en reprenant les deux
/// premiers nombres de la version du jeu : Minecraft 1.21.1 donne la série
/// 21.1.x. C'est la seule correspondance à connaître, et elle est stable
/// depuis l'abandon du versionnage hérité de Forge.
pub fn series_for(mc: &str) -> Option<String> {
    let mut parts = mc.split('.');
    if parts.next()? != "1" {
        return None;
    }
    let major = parts.next()?;
    let minor = parts.next().unwrap_or("0");
    Some(format!("{major}.{minor}."))
}

/// Dernière version publiée de NeoForge pour une version de Minecraft.
pub async fn latest_for(mc: &str, dl: &Downloader) -> Result<String> {
    let series = series_for(mc)
        .with_context(|| format!("aucune série NeoForge ne correspond à Minecraft {mc}"))?;

    let list: VersionList = serde_json::from_slice(&dl.bytes(VERSIONS_API).await?)
        .context("liste des versions NeoForge illisible")?;

    // Les versions sont publiées dans l'ordre ; on trie tout de même par
    // numéro de correctif, une republication pouvant désordonner la liste.
    let mut matching: Vec<(u32, String)> = list
        .versions
        .into_iter()
        .filter(|v| v.starts_with(&series))
        .filter(|v| !v.contains("beta"))
        .filter_map(|v| {
            let patch = v.rsplit('.').next()?.parse().ok()?;
            Some((patch, v))
        })
        .collect();
    matching.sort_by_key(|(patch, _)| *patch);

    matching
        .pop()
        .map(|(_, v)| v)
        .with_context(|| format!("aucune version NeoForge {series}x publiée"))
}

#[cfg(test)]
#[path = "versions.test.rs"]
mod tests;
