//! The lockfile: what was actually installed, and why.
//!
//! The manifest says what we want, the lockfile says what we got. The gap
//! between the two is exactly what resolution decided: the version chosen
//! when the manifest imposed none, and the dependencies added on their own.
//!
//! It serves two purposes, and either alone would justify writing it:
//!
//! - **replaying an installation identically**, months later, when every
//!   version has moved. That's what `install --locked` does;
//! - **making legible what was added without being requested**. Six months
//!   on, nobody remembers whether a jar is there by choice or because
//!   something else required it — the `reason` line answers that.
//!
//! It's versioned alongside the manifest, and a change that isn't explained
//! in a review is a signal.

mod entries;
mod reading;
mod summary;
mod timestamp;

use serde::{Deserialize, Serialize};

pub use entries::{LockedLoader, LockedMissing, LockedMod};
pub(crate) use timestamp::now_utc;

/// The lockfile, **shaped like the manifest and richer**.
///
/// This is a deliberate constraint: the two files live side by side, are
/// read one after the other, and compare at a glance. Naming the same thing
/// differently in each costs something on every read. The pack's name used
/// to be called `pack` here and `name` there; a mod's source, `origin` here
/// and `source` there. The old names remain accepted on read — lockfiles
/// already published don't need to be rewritten to be read — but are no
/// longer produced.
///
/// What the lockfile adds to the manifest: the generation date, each mod's
/// resolved fields (digests, size, URL, reason for its presence), and the
/// dependencies nobody could supply.
///
/// What it carries over, down to the servers: a lockfile alone is enough to
/// install **and** to know where to connect. A third-party tool — mc-content's
/// CI, a server script — no longer needs both files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub schema: u32,
    /// Pack name. `pack` in lockfiles from before.
    #[serde(alias = "pack")]
    pub name: String,
    /// Pack version, as the manifest declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Generation date, in UTC.
    pub generated: String,
    pub minecraft: String,
    pub loader: LockedLoader,
    pub java: u32,
    /// The installation's generation number.
    ///
    /// The mechanism by which whoever publishes the pack can say: “don't
    /// catch up on this update by diffing, wipe and start over”.
    ///
    /// Normal operation is differential — digests are compared and only what
    /// changed is redownloaded. That's what's needed: a three-hundred-mod
    /// modpack weighs half a gigabyte, and redownloading it on every update
    /// would be unbearable.
    ///
    /// But some transitions can't be caught up that way. A renamed mod
    /// leaves its old jar in place, a config folder changes shape, a shader
    /// leaves residue that nothing references anymore. The differential
    /// mechanism only sees what the lockfile describes; it's blind to what
    /// the lockfile no longer describes.
    ///
    /// Incrementing this number then triggers a purge before installation.
    /// What gets erased is what the launcher placed — mods, shaders,
    /// resource packs. **Never the game instance**: neither saves, nor
    /// options, nor configurations the player modified. Losing a world to
    /// catch up on a mod rename would be a remedy worse than the disease.
    ///
    /// `#[serde(default)]` without `skip_serializing_if`: already-published
    /// files read as generation 0, and every file written from now on
    /// carries its own explicitly. A field absent on write would force a
    /// distinction between “never placed” and “placed at zero”, when both
    /// mean the same thing.
    #[serde(default)]
    pub generation: u32,
    /// Where to connect, per environment — carried over from the manifest as
    /// is.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub servers: std::collections::BTreeMap<String, crate::manifest::Server>,
    pub mods: Vec<LockedMod>,
    /// Dependencies no source could supply. Empty under normal conditions;
    /// non-empty, it's the first place to look when the game refuses to
    /// start.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<LockedMissing>,
}
