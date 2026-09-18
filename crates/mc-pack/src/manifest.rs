//! The manifest: what a pack is, written once and versioned.
//!
//! A single file describes the entire installation — game version, loader,
//! Java version, mod list. It is deliberately short: everything that can be
//! deduced is, and what is written by hand is what a human decided.
//!
//! Each mod can be left free (“the latest compatible version”) or **pinned**
//! to an exact build. Both have their place: the development pack follows
//! updates, the production one only moves when someone decides it should.
//! That's the role of `file`, which designates an exact build.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod control;
mod entries;
#[cfg(test)]
mod fixtures;
mod reading;
mod server;
mod servers;

pub use entries::{Loader, ModEntry};
pub use server::Server;

/// Format version. A manifest from another version refuses itself rather
/// than being read halfway.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Minecraft version, e.g. `1.21.1`.
    pub minecraft: String,
    pub loader: Loader,
    /// Major Java version. Defaults to whatever Mojang requires for this
    /// game version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<u32>,
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
    #[serde(default)]
    pub mods: Vec<ModEntry>,
    /// Where to connect, per environment.
    ///
    /// The manifest is the same everywhere: it's the same content image,
    /// served under three names. So it isn't the one that chooses, it's the
    /// client — with its own environment, the one CI froze into it at build
    /// time. A dev launcher joins the dev server.
    ///
    /// The keys are those of `mc_log::Environment`: `development`,
    /// `preproduction`, `production`. Absence isn't an error — preproduction
    /// has no Minecraft servers behind it, and the game launches there
    /// without joining anything.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub servers: BTreeMap<String, Server>,
}
