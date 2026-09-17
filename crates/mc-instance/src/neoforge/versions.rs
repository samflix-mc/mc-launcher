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
///
/// Hors de portée des tests de mutation : cette fonction ne fait que
/// télécharger la liste publiée par NeoForge, à une adresse écrite dans ce
/// module. Le choix qu'elle en tire se vérifie — voir [`derniere_stable`].
#[mutants::skip]
pub async fn latest_for(mc: &str, dl: &Downloader) -> Result<String> {
    let series = series_for(mc)
        .with_context(|| format!("aucune série NeoForge ne correspond à Minecraft {mc}"))?;

    let list: VersionList = serde_json::from_slice(&dl.bytes(VERSIONS_API).await?)
        .context("liste des versions NeoForge illisible")?;

    derniere_stable(list.versions, &series)
        .with_context(|| format!("aucune version NeoForge {series}x publiée"))
}

/// La dernière version stable d'une série, parmi celles que NeoForge publie.
///
/// Trois règles, et chacune compte. La série d'abord : une version pour une
/// autre Minecraft ne démarrera pas. Les bêtas ensuite, écartées — elles
/// paraissent dans la même liste, et en installer une par inadvertance change
/// le jeu sous les pieds des joueurs. Le tri par numéro de correctif enfin :
/// les versions sont publiées dans l'ordre, mais une republication peut
/// désordonner la liste, et c'est la plus récente qu'on veut.
fn derniere_stable(versions: Vec<String>, series: &str) -> Option<String> {
    let mut matching: Vec<(u32, String)> = versions
        .into_iter()
        .filter(|v| v.starts_with(series))
        .filter(|v| !v.contains("beta"))
        .filter_map(|v| {
            let patch = v.rsplit('.').next()?.parse().ok()?;
            Some((patch, v))
        })
        .collect();
    matching.sort_by_key(|(patch, _)| *patch);

    matching.pop().map(|(_, v)| v)
}

#[cfg(test)]
#[path = "versions.test.rs"]
mod tests;
