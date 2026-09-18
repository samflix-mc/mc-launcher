//! Read a `version.json`, and walk up its inheritance chain.

use anyhow::{Context, Result, bail};
use std::path::Path;

use super::VersionJson;

fn read_version(shared: &Path, id: &str) -> Result<VersionJson> {
    let path = shared.join("versions").join(id).join(format!("{id}.json"));
    let raw = std::fs::read(&path)
        .with_context(|| format!("{id} is not installed: {} missing", path.display()))?;
    serde_json::from_slice(&raw).with_context(|| format!("{} unreadable", path.display()))
}

/// Full descriptor for a version, its base merged in.
///
/// The inheritance chain is followed several levels deep out of caution,
/// even though NeoForge only ever has one — nothing guarantees another
/// loader will stop there.
pub(in crate::launch) fn resolve_chain(shared: &Path, id: &str) -> Result<Vec<VersionJson>> {
    let mut chain = Vec::new();
    let mut current = id.to_string();
    for _ in 0..8 {
        let version = read_version(shared, &current)?;
        let parent = version.inherits_from.clone();
        chain.push(version);
        match parent {
            Some(next) => current = next,
            None => return Ok(chain),
        }
    }
    bail!("version inheritance chain too deep starting from {id}")
}

// --- Construction ------------------------------------------------------------

/// Uniqueness key for a library: group, artifact, and classifier.
///
/// The version is deliberately excluded — that's what makes it possible to
/// spot that NeoForge replaces a Mojang library with a different version.
pub(in crate::launch) fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.len() {
        0 | 1 => name.to_string(),
        2 | 3 => format!("{}:{}", parts[0], parts[1]),
        _ => format!("{}:{}:{}", parts[0], parts[1], parts[3]),
    }
}

#[cfg(test)]
#[path = "chain.test.rs"]
mod tests;
