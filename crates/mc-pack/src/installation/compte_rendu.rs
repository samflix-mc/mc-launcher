//! Ce que l'installation rend à celui qui l'a lancée.

use std::path::PathBuf;

use crate::Outcome;
use crate::lockfile::Lockfile;
use crate::source::Source;

use super::mods::Pose;

#[allow(clippy::too_many_arguments)]
pub(super) fn assembler(
    source: &Source,
    pose: Pose,
    java: mc_java::Java,
    game: mc_instance::vanilla::Vanilla,
    lock: Lockfile,
    lock_path: PathBuf,
    previous_lock: Option<Lockfile>,
    neoforge: String,
    from_cache: bool,
    ecarts: Vec<String>,
    purge: crate::etat::Purge,
) -> Outcome {
    Outcome {
        instance: pose.instance,
        server_dir: pose.server_dir,
        java,
        neoforge,
        assets_downloaded: game.assets_downloaded,
        libraries: game.libraries.len(),
        client_mods: pose.client_mods,
        server_mods: pose.server_mods,
        removed: pose.removed,
        lock,
        lock_path,
        previous_lock,
        source: source.describe(),
        from_cache,
        ecarts,
        purge,
    }
}
