//! Full sequence: from a JSON manifest to a playable instance.
//!
//! **Seven steps**, and [`Step::ALL`] is the proof of it — this comment used
//! to announce six, forgetting the first one, even though a display draws
//! the whole path from this very list. The order isn't arbitrary: each one
//! depends on the previous.
//!
//! 1. **the pack** — the manifest, and the lockfile if there is one; this is
//!    also where a purge gets decided, when the published generation has
//!    changed;
//! 2. **the loader** — `latest` is resolved right away, so the lockfile
//!    records an exact version and not a word;
//! 3. **Mojang's files** — they also give the Java version this game version
//!    requires;
//! 4. **Java** — detected or installed, at the EXACT major version the
//!    lockfile carries;
//! 5. **NeoForge** — its installer patches the vanilla client and needs the
//!    Java from the previous step;
//! 6. **the mods** — resolved, downloaded, then split between client and
//!    server;
//! 7. **the lockfile** — written last, it describes what was actually done.

mod coherence;
pub mod comparison;
mod installation;
pub mod state;
mod verification;

pub mod game;
pub mod lockfile;
pub mod manifest;
pub mod progress;
pub mod source;

pub use coherence::missing_client_mods;
pub use comparison::{Action, Drift, PackState, compare, presence, published_lock};
pub use game::{GameSession, Identity, UpdateOutcome, play, prepare, update, update_and_play};
pub use installation::install;
pub use progress::{Report, Silent, Step};
pub use state::{Before, LocalState, Purge};
pub use verification::verify;

use lockfile::Lockfile;
use std::path::PathBuf;

/// What an installation produced, for the report.
#[derive(Debug)]
pub struct Outcome {
    pub instance: mc_instance::Instance,
    pub server_dir: PathBuf,
    pub java: mc_java::Java,
    pub neoforge: String,
    pub assets_downloaded: usize,
    pub libraries: usize,
    pub client_mods: usize,
    pub server_mods: usize,
    pub removed: Vec<String>,
    pub lock: Lockfile,
    pub lock_path: PathBuf,
    pub previous_lock: Option<Lockfile>,
    /// Where the pack came from, as it was requested.
    pub source: String,
    /// The remote pack was unreachable and the local copy was used.
    pub from_cache: bool,
    /// What the installation drifts from when replaying the lockfile. Empty
    /// when the two match one for one — and always empty outside a replay,
    /// where there's nothing to compare against.
    pub drifts: Vec<String>,
    /// What the purge erased before installing, if there was a purge.
    ///
    /// In the REPORT and not in a progress note: `Report::note` is a single
    /// slot, overwritten unconditionally, and seven later notes overwrite it
    /// within the following second. A durable fact doesn't travel in a
    /// transient field — that's already why `missing` and `drifts` are here.
    pub purge: crate::state::Purge,
}

#[derive(Debug, Default)]
pub struct Options {
    /// Replay the lockfile exactly instead of resolving versions.
    pub locked: bool,
    /// Also install a full NeoForge server, not just its mods.
    pub with_server: bool,
    /// Instance name; by default, the pack's own.
    pub instance_name: Option<String>,
    pub layout: mc_instance::Layout,
}

#[cfg(test)]
mod fixtures;
