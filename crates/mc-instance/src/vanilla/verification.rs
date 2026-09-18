//! Recheck what's already installed, and say what the JVM needs.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use super::descriptor::{AssetIndex, Library};
use super::platform::{maven_path, mojang_arch, mojang_os};
use super::rules::allowed;

/// Rechecks the digest of every already-installed asset object.
///
/// Complement to [`Check::Quick`]: the installer only compares sizes, this
/// function does the exhaustive pass when certainty is wanted.
pub fn verify_assets(shared: &Path, index_id: &str) -> Result<VerifyReport> {
    let index_path = shared
        .join("assets")
        .join("indexes")
        .join(format!("{index_id}.json"));
    let parsed: AssetIndex =
        serde_json::from_slice(&std::fs::read(&index_path)?).context("unreadable asset index")?;

    let objects = shared.join("assets").join("objects");
    let mut report = VerifyReport::default();
    for object in parsed.objects.values() {
        let path = objects.join(&object.hash[..2]).join(&object.hash);
        if !path.is_file() {
            report.missing.push(object.hash.clone());
            continue;
        }
        match mc_dl::sha1_of_file(&path) {
            Ok(sum) if sum.eq_ignore_ascii_case(&object.hash) => report.ok += 1,
            _ => report.corrupt.push(object.hash.clone()),
        }
    }
    Ok(report)
}

#[derive(Debug, Default)]
pub struct VerifyReport {
    pub ok: usize,
    pub missing: Vec<String>,
    pub corrupt: Vec<String>,
}

impl VerifyReport {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.corrupt.is_empty()
    }
}

/// Minimal view of a descriptor, limited to its libraries.
///
/// The descriptor NeoForge produces carries neither `assetIndex` nor
/// `downloads`: it completes the version it designates via `inheritsFrom`.
/// Reading it with the full structure would fail, even though its fifty
/// libraries matter just as much as Mojang's.
#[derive(Debug, Deserialize)]
struct LibrariesOnly {
    libraries: Vec<Library>,
}

/// Libraries kept for this system, paths relative to the shared store.
///
/// Applies alike to Mojang's descriptor and to NeoForge's.
pub fn classpath(version_json: &Path, shared: &Path) -> Result<Vec<PathBuf>> {
    let version: LibrariesOnly = serde_json::from_slice(&std::fs::read(version_json)?)
        .with_context(|| format!("{} unreadable", version_json.display()))?;
    let os = mojang_os();
    let arch = mojang_arch();
    let root = shared.join("libraries");

    let mut out = Vec::new();
    for lib in &version.libraries {
        if !allowed(&lib.rules, os, arch) {
            continue;
        }
        let Some(path) = lib
            .downloads
            .as_ref()
            .and_then(|d| d.artifact.as_ref())
            .and_then(|a| a.path.clone())
            .or_else(|| maven_path(&lib.name))
        else {
            bail!("library with no usable path: {}", lib.name);
        };
        out.push(root.join(path));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "verification.test.rs"]
mod tests;
