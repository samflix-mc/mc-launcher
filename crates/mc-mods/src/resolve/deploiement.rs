//! Du cache au dossier `mods` de l'instance.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

use crate::jar::Side;

use super::plan::Plan;

/// resterait chargé et ferait diverger le registre du serveur.
pub fn deploy(plan: &Plan, side: Side, mods_dir: &Path) -> Result<Deployed> {
    std::fs::create_dir_all(mods_dir)
        .with_context(|| format!("création de {}", mods_dir.display()))?;

    let wanted: BTreeSet<String> = plan
        .for_side(side)
        .map(|m| m.candidate.file_name.clone())
        .collect();

    let mut removed = Vec::new();
    for entry in std::fs::read_dir(mods_dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jar") || wanted.contains(&name) {
            continue;
        }
        std::fs::remove_file(entry.path())?;
        removed.push(name);
    }

    let mut installed = 0;
    for entry in plan.for_side(side) {
        let dest = mods_dir.join(&entry.candidate.file_name);
        if dest.exists() {
            let same = match entry.candidate.checksum() {
                Some(attendue) => std::fs::read(&dest)
                    .map(|bytes| attendue.matches(&bytes))
                    .unwrap_or(false),
                None => true,
            };
            if same {
                installed += 1;
                continue;
            }
            std::fs::remove_file(&dest)?;
        }
        link_or_copy(&entry.path, &dest)?;
        installed += 1;
    }

    Ok(Deployed { installed, removed })
}

#[derive(Debug)]
pub struct Deployed {
    pub installed: usize,
    pub removed: Vec<String>,
}

/// Lien matériel si possible, copie sinon.
///
/// Un pack pèse plusieurs centaines de mégaoctets et le même jar sert souvent
/// au client et au serveur : le lien évite de le stocker trois fois. Il échoue
/// entre systèmes de fichiers différents, d'où le repli.
fn link_or_copy(from: &Path, to: &Path) -> Result<()> {
    if std::fs::hard_link(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)
        .with_context(|| format!("copie de {} vers {}", from.display(), to.display()))?;
    Ok(())
}
