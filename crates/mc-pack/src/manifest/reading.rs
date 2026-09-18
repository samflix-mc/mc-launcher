//! Reading a manifest, writing it, and what can be drawn from it.

use std::path::Path;

use anyhow::{Context, Result};
use mc_mods::Request;

use super::{Manifest, ModEntry};

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest> {
        let raw =
            std::fs::read(path).with_context(|| format!("reading manifest {}", path.display()))?;
        Manifest::parse(&raw).with_context(|| format!("unreadable manifest {}", path.display()))
    }

    /// Reads a manifest that has no path — one from an HTTP response.
    ///
    /// The check is the same as for a file: what arrives from the network
    /// deserves less trust, not more.
    pub fn parse(raw: &[u8]) -> Result<Manifest> {
        let manifest: Manifest = serde_json::from_slice(raw)?;
        manifest.check()?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        mc_dl::write_atomic(path, json.as_bytes())
    }

    /// Rejects an inconsistent manifest before any write to disk.
    ///
    /// A faulty manifest discovered after 800 MB of download costs far more
    /// than a check on read.
    pub fn requests(&self) -> Result<Vec<Request>> {
        self.mods.iter().map(ModEntry::to_request).collect()
    }

    /// Major Java version to guarantee.
    ///
    /// Three sources, in this order, and priority is the rule: what the
    /// MANIFEST declares wins over what Mojang requires, because a pack may
    /// have good reasons to impose a different major version than the bare
    /// game — a mod that only compiles with it, a JVM bug to work around.
    ///
    /// `Option` on input: descriptors from before 1.17 have no
    /// `javaVersion` block. The final 21 is the only place in the repo
    /// where this number is hardcoded, and it's here rather than in three
    /// call sites.
    pub fn java_major(&self, mojang_requires: Option<u32>) -> u32 {
        self.java.or(mojang_requires).unwrap_or(21)
    }
}

#[cfg(test)]
#[path = "reading.test.rs"]
mod tests;

#[cfg(test)]
#[path = "reading.requests.test.rs"]
mod tests_requests;
