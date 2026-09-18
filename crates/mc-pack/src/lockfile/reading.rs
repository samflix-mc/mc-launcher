//! Producing a lockfile from a plan, writing it, reading it back.

use std::path::Path;

use anyhow::{Context, Result};

use mc_mods::Plan;

use super::entries::{LockedMissing, LockedMod};
use super::timestamp::now_utc;
use super::{LockedLoader, Lockfile};

impl Lockfile {
    /// The lockfile for a plan, tied to the manifest that requested it.
    ///
    /// Takes the whole manifest rather than three of its fields: the
    /// lockfile carries over its name, version and servers, and that list
    /// will grow.
    pub fn from_plan(
        manifest: &crate::manifest::Manifest,
        loader: LockedLoader,
        java: u32,
        plan: &Plan,
    ) -> Lockfile {
        Lockfile {
            schema: crate::manifest::SCHEMA,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            generated: now_utc(),
            minecraft: manifest.minecraft.clone(),
            loader,
            java,
            // The generation comes from the MANIFEST, never from the
            // previous lockfile: it's whoever publishes who decides on a
            // purge, and the lockfile alongside it only carries that
            // decision to the player's machine.
            generation: manifest.generation,
            servers: manifest.servers.clone(),
            mods: plan
                .mods
                .iter()
                .map(|m| LockedMod {
                    slug: m.candidate.slug.clone(),
                    name: m.candidate.name.clone(),
                    source: m.candidate.origin,
                    project: m.candidate.project_id.clone(),
                    file: m.candidate.version_id.clone(),
                    version: m.candidate.version_number.clone(),
                    channel: m.candidate.channel,
                    file_name: m.candidate.file_name.clone(),
                    url: m.candidate.url.clone(),
                    sha1: m.candidate.sha1.clone(),
                    sha512: m.candidate.sha512.clone(),
                    size: m.candidate.size,
                    side: m.side.as_str().to_string(),
                    reason: m.reason.describe(),
                    // Root *and* embedded: this field is used by `verify`
                    // to decide a dependency is satisfied, never to
                    // recognize a duplicate. The resolver, for its part,
                    // distinguishes the two — two mods legitimately
                    // embedding the same library is fine.
                    provides: m.supplies().cloned().collect(),
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
            std::fs::read(path).with_context(|| format!("reading lockfile {}", path.display()))?;
        Lockfile::parse(&raw).with_context(|| format!("unreadable lockfile {}", path.display()))
    }

    /// Reads a lockfile that has no path — one from an HTTP response.
    pub fn parse(raw: &[u8]) -> Result<Lockfile> {
        Ok(serde_json::from_slice(raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        mc_dl::write_atomic(path, self.canonical()?.as_bytes())
    }

    /// The write form, once and for all.
    ///
    /// Extracted from `save` because [`Self::digest`] must produce exactly
    /// the same string: it's the only way a lockfile reread from disk and a
    /// lockfile received from the network give the same digest when they
    /// describe the same installation.
    fn canonical(&self) -> Result<String> {
        let mut json = serde_json::to_string_pretty(self)?;
        // A file that ends with a blank line reads better in a review — and
        // the lockfile is versioned alongside the manifest.
        json.push('\n');
        Ok(json)
    }

    /// What's compared to decide whether to reinstall.
    ///
    /// ## Why on the canonical form and not the received bytes
    ///
    /// The published lockfile arrives over HTTP, and the host makes no
    /// promise about formatting: two spaces of indentation today, four
    /// tomorrow, or minification the day someone plugs in a proxy. Hashing
    /// the received bytes would conclude “the pack changed” over an
    /// indentation difference, and eight hundred megabytes would download
    /// all over again.
    ///
    /// So both sides are reserialized with the same function. What's
    /// compared is then what the lockfile SAYS, not how it's written.
    ///
    /// Consequence worth knowing: a field added to the struct changes the
    /// digest of every lockfile, including ones that haven't moved. That's
    /// the intended behavior — a new field means the launcher knows
    /// something new about the installation, and one extra check costs less
    /// than a missing one.
    pub fn digest(&self) -> Result<String> {
        Ok(mc_dl::sha512_of_bytes(self.canonical()?.as_bytes()))
    }

    /// Path of the lockfile tied to a manifest: `samflix.json` gives
    /// `samflix.lock.json`, side by side in the repo.
    pub fn path_for(manifest: &Path) -> std::path::PathBuf {
        let stem = manifest
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "pack".to_string());
        manifest.with_file_name(format!("{stem}.lock.json"))
    }
}

#[cfg(test)]
#[path = "reading.test.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "reading.suite.test.rs"]
mod suite;
