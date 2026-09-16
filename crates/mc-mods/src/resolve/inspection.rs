//! Ce que les jars exigent vraiment, une fois ouverts.

use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

use crate::jar::Side;
use crate::Origin;

use super::plan::Installed;

/// Lit le descripteur de chaque jar fraîchement téléchargé.
pub(super) fn inspect_all(chosen: &mut BTreeMap<(Origin, String), Installed>) -> Result<()> {
    for entry in chosen.values_mut() {
        if !entry.provides.is_empty() || entry.path.as_os_str().is_empty() {
            continue;
        }
        let info = crate::jar::inspect(&entry.path)?;
        entry.provides = info.provides;
        entry.requires = info.requires;
    }
    Ok(())
}

/// Écarte les jars qui fournissent un `modId` déjà fourni par un autre.
///
/// Le même mod peut arriver deux fois par deux chemins : demandé par son slug
/// Modrinth, et tiré comme dépendance par son identifiant CurseForge. Les
/// clés de projet diffèrent, donc rien ne les rapproche — sauf le `modId` que
/// les deux jars déclarent. Or deux jars du même `modId` dans `mods` font
/// échouer NeoForge au chargement, avec un message qui n'aide pas.
///
/// En cas de doublon, on garde celui qui a une empreinte publiée, puis le mod


/// `modId` exigés par au moins un jar et fournis par aucun.
pub(super) fn missing_requirements(
    chosen: &BTreeMap<(Origin, String), Installed>,
) -> Vec<(String, String, Side)> {
    let provided: BTreeSet<&String> = chosen.values().flat_map(|m| m.provides.iter()).collect();

    let mut missing: BTreeMap<String, (String, Side)> = BTreeMap::new();
    for entry in chosen.values() {
        for requirement in &entry.requires {
            if provided.contains(&requirement.mod_id) {
                continue;
            }
            let slot = missing
                .entry(requirement.mod_id.clone())
                .or_insert_with(|| (entry.candidate.name.clone(), requirement.side));
            slot.1 = slot.1.union(requirement.side);
        }
    }

    missing
        .into_iter()
        .map(|(mod_id, (by, side))| (mod_id, by, side))
        .collect()
}

#[cfg(test)]
#[path = "inspection.test.rs"]
mod tests;
