//! Open a jar, including the jars it bundles.

use anyhow::{Context, Result};
use std::io::{Cursor, Read};
use std::path::Path;

use super::descriptor::{JarInfo, parse_descriptor};

/// Reads a jar from disk.
pub fn inspect(path: &Path) -> Result<JarInfo> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    inspect_bytes(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// Reads a jar already in memory, descending into its bundled jars.
pub fn inspect_bytes(bytes: &[u8]) -> Result<JarInfo> {
    let mut info = JarInfo::default();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;

    // NeoForge 1.20.5+ uses neoforge.mods.toml; mods.toml is still read for
    // jars published before the rename, still numerous in 1.21.
    let descriptor = read_entry(&mut archive, "META-INF/neoforge.mods.toml")
        .or_else(|| read_entry(&mut archive, "META-INF/mods.toml"));

    if let Some(text) = descriptor {
        let parsed = parse_descriptor(&String::from_utf8_lossy(&text))?;
        info.provides.extend(parsed.provides);
        info.requires.extend(parsed.requires);
    }

    for nested in embedded_jars(&mut archive)? {
        // Only the `modId`s provided matter: a bundled library's own
        // dependencies are, by construction, satisfied by the mod that
        // bundles it.
        //
        // They join `bundled`, not `provides`: this jar brings them in, but
        // they don't say who it is. Two mods legitimately bundle the same
        // library — a jar bundled by a bundled jar is still a contribution,
        // hence reusing `provided` and not just `provides`.
        if let Ok(sub) = inspect_bytes(&nested) {
            info.bundled.extend(sub.provided().cloned());
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

/// Jars bundled by JarJar, listed in `META-INF/jarjar/metadata.json`.
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
#[path = "reading.test.rs"]
mod tests;
