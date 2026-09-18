//! Does the installed instance actually match this lock?

use anyhow::{Result, bail};

use crate::lockfile::Lockfile;

/// The lock comes from this pack's cache, which is split by host; the
/// instance, though, carries the pack's name and there's only one for all
/// three environments. An `install` run with a different `SAMFLIX_ENV` could
/// therefore have replaced these jars without this lock knowing anything
/// about it.
///
/// Starting anyway means letting the server settle it with an ejection for
/// mismatched mod lists — and that ejection doesn't name its cause.
pub(crate) fn verify(lock: &Lockfile, instance: &mc_instance::Instance) -> Result<()> {
    let missing = crate::missing_client_mods(lock, instance);
    if missing.is_empty() {
        return Ok(());
    }
    bail!(
        "{} mods from the lock are missing in {}: this instance was installed \
         from a different pack than this one.\n\
         Rerun \"mc-pack install\" — environment \"{}\", {}.\n  {}",
        missing.len(),
        instance.mods_dir().display(),
        mc_log::environment::current().as_str(),
        mc_log::environment::origin(),
        missing.join("\n  ")
    )
}

#[cfg(test)]
#[path = "coherence.test.rs"]
mod tests;
