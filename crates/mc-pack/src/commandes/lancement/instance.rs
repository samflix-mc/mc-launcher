//! Le pack posé sur cette machine, et l'instance qui lui correspond.

use anyhow::{Result, anyhow};

use mc_pack::lockfile::Lockfile;
use mc_pack::manifest::Manifest;
use mc_pack::source::Source;

use super::coherence;

/// Ouvre ce qui est installé, pas ce qui est publié.
///
/// Aller chercher le pack du jour décrirait des mods que le dossier ne contient
/// pas, et interdirait de jouer sans réseau.
pub(super) fn ouvrir(
    source: &Source,
    options: &mc_pack::Options,
) -> Result<(Manifest, Lockfile, mc_instance::Instance)> {
    let pack = source.load_local()?;
    let manifest = pack.manifest;
    let lock = pack.lock.ok_or_else(|| {
        anyhow!(
            "{} absent : lancer « mc-pack install » avant de jouer",
            pack.lock_path.display()
        )
    })?;

    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(&manifest.name));

    coherence::verifier(&lock, &instance)?;
    Ok((manifest, lock, instance))
}

#[cfg(test)]
#[path = "instance.test.rs"]
mod tests;
