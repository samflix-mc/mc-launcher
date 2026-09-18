//! The manifest that `lock` is allowed to work on.

use std::path::PathBuf;

use anyhow::{Result, bail};

use mc_pack::manifest::Manifest;
use mc_pack::source::Source;

pub(super) fn manifest_to_lock(source: &Source) -> Result<(PathBuf, Manifest)> {
    // Resolving produces a lock, and a lock has to be placed somewhere. A
    // published pack doesn't offer that place — and doesn't need it: it
    // already arrives locked, which is exactly what makes it a published
    // pack.
    let Some(manifest_path) = source.local_path() else {
        bail!(
            "lock works on a manifest to edit: give it a path.\n\
             The published pack is already locked — mc-content resolves it."
        );
    };
    let manifest = Manifest::load(manifest_path)?;

    // This is the one and only place an unreadable "servers" key gets
    // rejected. Reading the manifest can't take care of it: that also
    // applies to the downloaded pack, and a binary that refused an
    // environment it doesn't know would stop the day mc-content declares
    // one more. lock is the opposite — the command run before publishing,
    // on the file you just wrote, with the author watching the screen.
    let problems = manifest.server_problems();
    if !problems.is_empty() {
        bail!(
            "{}: some \"servers\" keys would never be read —\n  {}",
            manifest_path.display(),
            problems.join("\n  ")
        );
    }

    Ok((manifest_path.to_path_buf(), manifest))
}

#[cfg(test)]
#[path = "preparation.test.rs"]
mod tests;
