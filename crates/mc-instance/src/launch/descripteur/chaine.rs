//! Lire un `version.json`, et remonter sa chaîne d'héritage.

use anyhow::{bail, Context, Result};
use std::path::Path;

use super::VersionJson;

fn read_version(shared: &Path, id: &str) -> Result<VersionJson> {
    let path = shared.join("versions").join(id).join(format!("{id}.json"));
    let raw = std::fs::read(&path)
        .with_context(|| format!("{id} n'est pas installé : {} absent", path.display()))?;
    serde_json::from_slice(&raw).with_context(|| format!("{} illisible", path.display()))
}

/// Descripteur complet d'une version, son socle fusionné.
///
/// La chaîne d'héritage est suivie sur plusieurs niveaux par prudence, même si
/// NeoForge n'en compte qu'un — rien ne garantit qu'un autre chargeur s'en
/// tienne là.
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
    bail!("chaîne d'héritage de versions trop profonde à partir de {id}")
}

// --- Construction ------------------------------------------------------------

/// Clé d'unicité d'une bibliothèque : groupe, artefact et classifier.
///
/// La version en est exclue volontairement — c'est ce qui permet de repérer
/// que NeoForge remplace une bibliothèque de Mojang par une autre version.
pub(in crate::launch) fn library_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.len() {
        0 | 1 => name.to_string(),
        2 | 3 => format!("{}:{}", parts[0], parts[1]),
        _ => format!("{}:{}:{}", parts[0], parts[1], parts[3]),
    }
}

#[cfg(test)]
#[path = "chaine.test.rs"]
mod tests;
