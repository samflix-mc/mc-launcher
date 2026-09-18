//! Where the pack comes from: a file in the repo, or an address.
//!
//! The two cases don't serve the same people, and that's what decides their
//! behavior.
//!
//! **A file** is what you edit. The manifest says what you want, resolution
//! looks up the versions, and the lock is rewritten next to it. It's the
//! development gesture, the one that moves the pack forward.
//!
//! **An address** is what you receive. The manifest and its lock are
//! downloaded together, and the lock is **replayed as-is**: a player resolves
//! nothing. If they did, their machine would pick its own versions the day a
//! mod publishes a new one, and they'd show up on the server with NeoForge
//! registries that no longer match — an ejection at connect, with no useful
//! message.
//!
//! The remote lock is therefore the only source of truth on the player's
//! side, and that's exactly what mc-content publishes.
//!
//! ## Offline
//!
//! Every successful download leaves a copy in the cache. When the network is
//! missing, that copy is picked back up and the user is warned: playing with
//! yesterday's pack beats not playing. Nothing is cached before it's been
//! read back — a truncated response or an HTML error page would otherwise
//! replace a valid pack with nothing.

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use std::path::PathBuf;

/// Address of the pack in production. mc-launcher-site serves the `launcher/`
/// directory of the mc-content image, which is where the mod list
mod cache;
mod local;
mod reading;
mod remote;

pub use remote::{URL_DEVELOPMENT, URL_PREPRODUCTION, URL_PRODUCTION, default_url};

// Exposed for comparison, which must derive the lock's address the same way
// fetch does. Copying these three lines elsewhere would reintroduce exactly
// the defect this module exists to avoid.
pub use cache::lock_url_for;

#[derive(Debug, Clone)]
pub enum Source {
    /// A path on disk. The lock sits next to it, and will be rewritten.
    File { manifest: PathBuf },
    /// A URL. The manifest and the lock are downloaded then cached.
    Remote { url: String, cache_dir: PathBuf },
}

/// A read pack, whatever its origin.
#[derive(Debug)]
pub struct Pack {
    pub manifest: Manifest,
    /// Already-known lock: the repo's, or the one just downloaded. Absent the
    /// first time a local pack is resolved.
    pub lock: Option<Lockfile>,
    /// Where to write the lock, when there's reason to write it.
    pub lock_path: PathBuf,
    /// The lock is authoritative: its builds are replayed instead of looked up.
    pub replay: bool,
    /// The network was missing and the cache took over.
    pub from_cache: bool,
}
