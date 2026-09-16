//! Lire le TOML que NeoForge lit.

use anyhow::{Context, Result};

use super::{is_platform, JarInfo, Requirement};
use crate::jar::cote::Side;

/// Ce qu'un jar déclare fournir, et ce qu'il exige pour démarrer.
pub fn parse_descriptor(text: &str) -> Result<JarInfo> {
    let root: toml::Value = text.parse().context("neoforge.mods.toml illisible")?;
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
            // Un jar multi-mods exige souvent la même bibliothèque plusieurs
            // fois ; on fusionne les côtés plutôt que de dupliquer.
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

/// Une dépendance est-elle obligatoire ?
///
/// NeoForge 1.21 écrit `type = "required"`. Les jars antérieurs, et ceux portés
/// depuis Forge, écrivent `mandatory = true`. Les deux formes se croisent dans
/// un même modpack.
fn is_mandatory(dep: &toml::Value) -> bool {
    if let Some(kind) = dep.get("type").and_then(|v| v.as_str()) {
        return kind.eq_ignore_ascii_case("required");
    }
    dep.get("mandatory")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "analyse.test.rs"]
mod tests;
