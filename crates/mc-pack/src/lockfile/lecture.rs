//! Produire un verrou depuis un plan, l'écrire, le relire.

use std::path::Path;

use anyhow::{Context, Result};

use mc_mods::Plan;

use super::entrees::{LockedMissing, LockedMod};
use super::horodatage::now_utc;
use super::{LockedLoader, Lockfile};

impl Lockfile {
    pub fn from_plan(
        pack: &str,
        minecraft: &str,
        loader: LockedLoader,
        java: u32,
        plan: &Plan,
    ) -> Lockfile {
        Lockfile {
            schema: crate::manifest::SCHEMA,
            pack: pack.to_string(),
            generated: now_utc(),
            minecraft: minecraft.to_string(),
            loader,
            java,
            mods: plan
                .mods
                .iter()
                .map(|m| LockedMod {
                    slug: m.candidate.slug.clone(),
                    name: m.candidate.name.clone(),
                    origin: m.candidate.origin,
                    project: m.candidate.project_id.clone(),
                    file: m.candidate.version_id.clone(),
                    version: m.candidate.version_number.clone(),
                    file_name: m.candidate.file_name.clone(),
                    url: m.candidate.url.clone(),
                    sha1: m.candidate.sha1.clone(),
                    sha512: m.candidate.sha512.clone(),
                    size: m.candidate.size,
                    side: m.side.as_str().to_string(),
                    reason: m.reason.describe(),
                    provides: m.provides.iter().cloned().collect(),
                })
                .collect(),
            unresolved: plan
                .unresolved
                .iter()
                .map(|u| LockedMissing {
                    mod_id: u.mod_id.clone(),
                    required_by: u.required_by.clone(),
                    side: u.side.as_str().to_string(),
                })
                .collect(),
        }
    }

    pub fn load(path: &Path) -> Result<Lockfile> {
        let raw =
            std::fs::read(path).with_context(|| format!("lecture du verrou {}", path.display()))?;
        Lockfile::parse(&raw).with_context(|| format!("verrou {} illisible", path.display()))
    }

    /// Lit un verrou qui n'a pas de chemin — celui d'une réponse HTTP.
    pub fn parse(raw: &[u8]) -> Result<Lockfile> {
        Ok(serde_json::from_slice(raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        mc_dl::write_atomic(path, json.as_bytes())
    }

    /// Chemin du verrou associé à un manifeste : `samflix.json` donne
    /// `samflix.lock.json`, côte à côte dans le dépôt.
    pub fn path_for(manifest: &Path) -> std::path::PathBuf {
        let stem = manifest
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "pack".to_string());
        manifest.with_file_name(format!("{stem}.lock.json"))
    }
}

#[cfg(test)]
#[path = "lecture.test.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "lecture.suite.test.rs"]
mod suite;
