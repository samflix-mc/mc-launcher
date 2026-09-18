//! Which NeoForge version for which Minecraft version.

use anyhow::{Context, Result};
use mc_dl::Downloader;
use serde::Deserialize;

const VERSIONS_API: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
struct VersionList {
    versions: Vec<String>,
}

/// NeoForge series matching a Minecraft version.
///
/// NeoForge numbers `<major>.<minor>.<patch>` by reusing the first two
/// numbers of the game version: Minecraft 1.21.1 gives series 21.1.x. It's
/// the only mapping to know, and it's been stable since Forge's legacy
/// versioning was dropped.
pub fn series_for(mc: &str) -> Option<String> {
    let mut parts = mc.split('.');
    if parts.next()? != "1" {
        return None;
    }
    let major = parts.next()?;
    let minor = parts.next().unwrap_or("0");
    Some(format!("{major}.{minor}."))
}

/// Latest published NeoForge version for a Minecraft version.
///
/// Out of scope for mutation testing: this function only downloads the list
/// NeoForge publishes, at an address written into this module. What it picks
/// from it is verifiable — see [`latest_stable`].
#[mutants::skip]
pub async fn latest_for(mc: &str, dl: &Downloader) -> Result<String> {
    let series =
        series_for(mc).with_context(|| format!("no NeoForge series matches Minecraft {mc}"))?;

    let list: VersionList = serde_json::from_slice(&dl.bytes(VERSIONS_API).await?)
        .context("unreadable NeoForge version list")?;

    latest_stable(list.versions, &series)
        .with_context(|| format!("no NeoForge {series}x version published"))
}

/// The latest stable version of a series, among the ones NeoForge publishes.
///
/// Three rules, and each one matters. The series first: a version for a
/// different Minecraft won't start. Betas next, excluded — they appear in
/// the same list, and installing one by mistake changes the game under
/// players' feet. Sorting by patch number last: versions are published in
/// order, but a republish can disorder the list, and it's the most recent
/// one that's wanted.
fn latest_stable(versions: Vec<String>, series: &str) -> Option<String> {
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
