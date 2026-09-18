//! What the installation returns to whoever launched it.

use std::path::PathBuf;

use crate::Outcome;
use crate::lockfile::Lockfile;
use crate::source::Source;

use super::mods::Placement;

#[allow(clippy::too_many_arguments)]
pub(super) fn assemble(
    source: &Source,
    placement: Placement,
    java: mc_java::Java,
    game: mc_instance::vanilla::Vanilla,
    lock: Lockfile,
    lock_path: PathBuf,
    previous_lock: Option<Lockfile>,
    neoforge: String,
    from_cache: bool,
    drifts: Vec<String>,
    purge: crate::state::Purge,
) -> Outcome {
    Outcome {
        instance: placement.instance,
        server_dir: placement.server_dir,
        java,
        neoforge,
        assets_downloaded: game.assets_downloaded,
        libraries: game.libraries.len(),
        client_mods: placement.client_mods,
        server_mods: placement.server_mods,
        removed: placement.removed,
        lock,
        lock_path,
        previous_lock,
        source: source.describe(),
        from_cache,
        drifts,
        purge,
    }
}
