//! What resolution returns: the kept mods, and what's missing.

use crate::Candidate;
use crate::jar::Side;
use std::collections::BTreeSet;
use std::path::PathBuf;

use super::reason::Reason;

/// A resolved, downloaded and inspected mod.
#[derive(Debug, Clone)]
pub struct Installed {
    pub candidate: Candidate,
    /// Effective side, after combining the manifest, the project's metadata
    /// and what the jar's descriptor requires.
    pub side: Side,
    pub reason: Reason,
    /// Path in the launcher's cache.
    pub path: PathBuf,
    /// `modId`s declared by the jar's descriptor. **The mod's identity**,
    /// and the only basis on which two projects can be called redundant.
    pub provides: BTreeSet<String>,
    /// `modId`s brought by bundled jars (JarJar).
    ///
    /// They satisfy dependencies without defining an identity — see
    /// [`crate::jar::JarInfo::bundled`].
    pub bundled: BTreeSet<String>,
    /// `modId`s this jar requires to start, excluding the platform.
    pub requires: Vec<crate::jar::Requirement>,
    /// In the name of what this build holds its spot — see [`authority`].
    ///
    /// Carried by the kept entry rather than by a second table indexed the
    /// same way: two tables to keep in sync is an invitation to let them
    /// drift, and `reason` goes into the lock.
    pub(super) authority: u8,
}

/// Complete result of a resolution.
#[derive(Debug, Default)]
pub struct Plan {
    pub mods: Vec<Installed>,
    /// Dependencies a jar required that no source could supply.
    /// Non-blocking here: it's the caller who decides whether to stop or
    /// warn.
    pub unresolved: Vec<Unresolved>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub mod_id: String,
    pub required_by: String,
    pub side: Side,
}

impl Installed {
    /// Everything this mod supplies, root and bundled alike.
    ///
    /// This is what satisfies a dependency. Deduplication, on the other
    /// hand, only looks at [`provides`]: the two uses don't share the same
    /// semantics, and conflating them removed legitimate mods.
    ///
    /// [`provides`]: Installed::provides
    pub fn supplies(&self) -> impl Iterator<Item = &String> {
        self.provides.iter().chain(self.bundled.iter())
    }
}

impl Plan {
    pub fn for_side(&self, side: Side) -> impl Iterator<Item = &Installed> {
        self.mods.iter().filter(move |m| m.side.includes(side))
    }
}

#[cfg(test)]
#[path = "plan.test.rs"]
mod tests;
