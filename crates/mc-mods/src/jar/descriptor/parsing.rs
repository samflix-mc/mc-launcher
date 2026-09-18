//! Read the TOML that NeoForge reads.

use anyhow::{Context, Result};

use super::{JarInfo, Requirement, is_platform};
use crate::jar::side::Side;

/// What a jar declares it provides, and what it requires to start.
pub fn parse_descriptor(text: &str) -> Result<JarInfo> {
    let root: toml::Value = text.parse().context("unreadable neoforge.mods.toml")?;
    let mut info = JarInfo::default();

    if let Some(mods) = root.get("mods").and_then(|v| v.as_array()) {
        for entry in mods {
            if let Some(id) = entry.get("modId").and_then(|v| v.as_str()) {
                info.provides.insert(id.to_string());
            }
        }
    }

    let Some(dependencies) = root.get("dependencies").and_then(|v| v.as_table()) else {
        return Ok(info);
    };

    for declarations in dependencies.values() {
        let Some(list) = declarations.as_array() else {
            continue;
        };
        for dep in list {
            let Some(id) = dep.get("modId").and_then(|v| v.as_str()) else {
                continue;
            };
            if is_platform(id) {
                continue;
            }
            if !is_mandatory(dep) {
                continue;
            }
            let side = dep
                .get("side")
                .and_then(|v| v.as_str())
                .and_then(Side::parse)
                .unwrap_or(Side::Both);
            let requirement = Requirement {
                mod_id: id.to_string(),
                version_range: dep
                    .get("versionRange")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                side,
            };
            // A multi-mod jar often requires the same library several
            // times; we merge the sides instead of duplicating.
            match info
                .requires
                .iter_mut()
                .find(|r| r.mod_id == requirement.mod_id)
            {
                Some(existing) => existing.side = existing.side.union(requirement.side),
                None => info.requires.push(requirement),
            }
        }
    }

    Ok(info)
}

/// Is a dependency mandatory?
///
/// NeoForge 1.21 writes `type = "required"`. Earlier jars, and those ported
/// from Forge, write `mandatory = true`. Both forms cross paths within the
/// same modpack.
fn is_mandatory(dep: &toml::Value) -> bool {
    if let Some(kind) = dep.get("type").and_then(|v| v.as_str()) {
        return kind.eq_ignore_ascii_case("required");
    }
    dep.get("mandatory")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "parsing.test.rs"]
mod tests;
