//! The pack laid down on this machine, and the instance that matches it.

use anyhow::{Result, anyhow};

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use crate::source::Source;

use super::coherence;

/// Opens what's installed, not what's published.
///
/// Fetching today's pack would describe mods the folder doesn't contain, and
/// would forbid playing without a network.
pub(crate) fn open(
    source: &Source,
    options: &crate::Options,
) -> Result<(Manifest, Lockfile, mc_instance::Instance)> {
    let pack = source.load_local()?;
    let manifest = pack.manifest;
    let lock = pack.lock.ok_or_else(|| {
        anyhow!(
            "{} missing: run \"mc-pack install\" before playing",
            pack.lock_path.display()
        )
    })?;

    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(&manifest.name));

    coherence::verify(&lock, &instance)?;
    Ok((manifest, lock, instance))
}

#[cfg(test)]
#[path = "instance.test.rs"]
mod tests;
