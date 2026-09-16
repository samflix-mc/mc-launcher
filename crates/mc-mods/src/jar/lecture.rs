//! Ouvrir un jar, y compris les jars qu'il embarque.

use anyhow::{Context, Result};
use std::io::{Cursor, Read};
use std::path::Path;

use super::descripteur::{parse_descriptor, JarInfo};

/// Lit un jar sur le disque.
pub fn inspect(path: &Path) -> Result<JarInfo> {
    let bytes = std::fs::read(path).with_context(|| format!("lecture de {}", path.display()))?;
    inspect_bytes(&bytes).with_context(|| format!("analyse de {}", path.display()))
}

/// Lit un jar déjà en mémoire, en descendant dans ses jars embarqués.
pub fn inspect_bytes(bytes: &[u8]) -> Result<JarInfo> {
    let mut info = JarInfo::default();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;

    // NeoForge 1.20.5+ utilise neoforge.mods.toml ; mods.toml reste lu pour les
    // jars publiés avant le renommage, encore nombreux en 1.21.
    let descriptor = read_entry(&mut archive, "META-INF/neoforge.mods.toml")
        .or_else(|| read_entry(&mut archive, "META-INF/mods.toml"));

    if let Some(text) = descriptor {
        let parsed = parse_descriptor(&String::from_utf8_lossy(&text))?;
        info.provides.extend(parsed.provides);
        info.requires.extend(parsed.requires);
    }

    for nested in embedded_jars(&mut archive)? {
        // Seuls les `modId` fournis comptent : les dépendances d'une
        // bibliothèque embarquée sont, par construction, satisfaites par le
        // mod qui l'embarque.
        if let Ok(sub) = inspect_bytes(&nested) {
            info.provides.extend(sub.provides);
        }
    }

    Ok(info)
}

fn read_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<Vec<u8>> {
    let mut entry = archive.by_name(name).ok()?;
    let mut buffer = Vec::new();
    entry.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

/// Jars embarqués par JarJar, listés dans `META-INF/jarjar/metadata.json`.
fn embedded_jars<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<Vec<Vec<u8>>> {
    let Some(raw) = read_entry(archive, "META-INF/jarjar/metadata.json") else {
        return Ok(Vec::new());
    };
    let metadata: serde_json::Value = serde_json::from_slice(&raw)?;
    let Some(jars) = metadata.get("jars").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    for jar in jars {
        let Some(path) = jar.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Some(bytes) = read_entry(archive, path) {
            out.push(bytes);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "lecture.test.rs"]
mod tests;
