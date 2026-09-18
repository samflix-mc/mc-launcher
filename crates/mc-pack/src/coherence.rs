//! Does the installed instance match the lockfile we're reading?

use mc_mods::Side;

use crate::lockfile::Lockfile;

/// Mods the lockfile announces on the client side that the instance doesn't
/// have.
///
/// The lockfile and the instance don't come from the same place. The
/// lockfile is stored in the pack's cache, which is split by host: a dev pack
/// and a production pack don't step on each other. The instance, though, is
/// named after the pack — "samflix" in all three cases, since it's the same
/// content image served under three names — so there's only one for all
/// three.
///
/// An `install` run with a different SAMFLIX_ENV replaces this single
/// instance's jars without the other environment's lockfile knowing anything
/// about it. The two then silently contradict each other, and it's the
/// server that settles it, with a kick for diverging mod lists — a world away
/// from its actual cause.
///
/// Comparing file names is enough and only costs one `stat` per mod. Digests
/// are `verify`'s business, which is allowed to be slow.
pub fn missing_client_mods(lock: &Lockfile, instance: &mc_instance::Instance) -> Vec<String> {
    let mods_dir = instance.mods_dir();
    lock.mods
        .iter()
        .filter(|entry| {
            Side::parse(&entry.side)
                .unwrap_or(Side::Both)
                .includes(Side::Client)
        })
        .filter(|entry| !mods_dir.join(&entry.file_name).is_file())
        .map(|entry| entry.file_name.clone())
        .collect()
}

#[cfg(test)]
#[path = "coherence.test.rs"]
mod tests;
