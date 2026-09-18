//! Verify that an installation is complete and intact.

use anyhow::{Context, Result};
use mc_mods::Side;

use crate::Options;
use crate::source::Source;

pub fn verify(source: &Source, options: &Options, deep: bool) -> Result<Vec<String>> {
    let pack = source.load_local()?;
    let manifest = pack.manifest;
    let lock = pack.lock.with_context(|| {
        format!(
            "{} missing: nothing to verify until \"mc-pack install\" has run",
            pack.lock_path.display()
        )
    })?;

    let mut problems = mc_instance::verify(
        &manifest.minecraft,
        &lock.loader.version,
        &options.layout,
        deep,
    )?;

    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    let server_mods = instance.dir.join("server").join("mods");

    for entry in &lock.mods {
        let side = Side::parse(&entry.side).unwrap_or(Side::Both);
        let mut targets = Vec::new();
        if side.includes(Side::Client) {
            targets.push(instance.mods_dir().join(&entry.file_name));
        }
        if side.includes(Side::Server) {
            targets.push(server_mods.join(&entry.file_name));
        }

        for path in targets {
            if !path.is_file() {
                problems.push(format!("missing mod: {}", path.display()));
                continue;
            }
            // A lockfile with no digest at all can't be verified: that's the
            // case for entries written from a source that didn't publish
            // one, before we started computing them ourselves.
            let Some(expected) = entry.checksum() else {
                continue;
            };
            match std::fs::read(&path) {
                Ok(bytes) if expected.matches(&bytes) => {}
                Ok(bytes) => problems.push(format!(
                    "{}: digest {} instead of {}",
                    path.display(),
                    expected.of(&bytes),
                    expected.expected()
                )),
                Err(e) => problems.push(format!("{}: unreadable ({e})", path.display())),
            }
        }
    }

    // The lockfile carries the `modId`s each jar provides: the whole's
    // coherence is checked without reopening a single archive.
    let provided: std::collections::BTreeSet<&String> =
        lock.mods.iter().flat_map(|m| m.provides.iter()).collect();
    for missing in &lock.unresolved {
        if !provided.contains(&missing.mod_id) {
            problems.push(format!(
                "unmet dependency: {} required by {}",
                missing.mod_id, missing.required_by
            ));
        }
    }

    Ok(problems)
}

#[cfg(test)]
#[path = "verification.test.rs"]
mod tests;
