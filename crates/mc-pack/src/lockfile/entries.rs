//! What a lockfile line retains about a mod.

use mc_mods::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedLoader {
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMod {
    pub slug: String,
    /// Project display name — “Just Enough Items” for `jei`.
    pub name: String,
    /// Where the mod comes from. `origin` in lockfiles from before; `source`
    /// is the name the manifest uses for the same thing, and the two files
    /// are read one after the other.
    #[serde(alias = "origin")]
    pub source: Origin,
    /// Project identifier at its source.
    pub project: String,
    /// Build identifier. It's what makes replaying the installation
    /// possible.
    pub file: String,
    pub version: String,
    /// Channel of the retained build. A pre-release in a pack shows up
    /// here, and the manifest uses the same word.
    #[serde(default = "default_channel")]
    pub channel: mc_mods::Channel,
    pub file_name: String,
    /// Direct download URL, as given by the source.
    ///
    /// It makes the lockfile usable by something other than the launcher:
    /// mc-content's CI checks that it responds, and a server can install
    /// the jar without knowing anything about Modrinth. Tolerated absent,
    /// to read lockfiles written before it existed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    /// The strong digest, when the source publishes it — Modrinth — or when
    /// it was computed for lack of anything better. Absent from lockfiles
    /// written before it existed, hence the `default`: those stay readable,
    /// and their SHA-1 keeps being authoritative until the next `lock`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    pub size: u64,
    pub side: String,
    /// In plain terms: requested, declared dependency, or implicit
    /// dependency.
    pub reason: String,
    /// `modId`s this jar provides, embedded jars included. These are what
    /// satisfy other mods' dependencies.
    ///
    /// Deliberately the union of both: this field answers “is this
    /// dependency covered?”, never “are these two entries the same mod?”.
    /// The second question is settled solely on the descriptor's `modId`,
    /// and the resolver handles that — conflating the two removed
    /// legitimate mods from the pack.
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMissing {
    pub mod_id: String,
    pub required_by: String,
    pub side: String,
}

impl LockedMod {
    /// The strongest digest the lockfile carries for this jar.
    ///
    /// SHA-512 when Modrinth publishes it or it was computed for lack of
    /// anything better; SHA-1 for what only CurseForge gives, and for
    /// lockfiles written before the field existed.
    pub fn checksum(&self) -> Option<mc_dl::Checksum> {
        self.sha512
            .clone()
            .map(mc_dl::Checksum::Sha512)
            .or_else(|| self.sha1.clone().map(mc_dl::Checksum::Sha1))
    }
}

/// The channel of lockfiles written before this field existed.
///
/// `release` and not the real channel: a lockfile from before says nothing
/// about the channel, and assuming a pre-release would raise warnings on a
/// pack nobody touched.
fn default_channel() -> mc_mods::Channel {
    mc_mods::Channel::Release
}
