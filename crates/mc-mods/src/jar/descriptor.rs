//! `META-INF/neoforge.mods.toml`: what the jar declares about itself.
//!
//! It's the only source the game actually reads at startup — the one whose
//! mismatch with API metadata makes a client crash on "Missing or
//! unsupported mods".

use std::collections::BTreeSet;

use super::side::Side;

/// IDs that NeoForge always considers present: they describe the platform,
/// not a mod to install.
pub const PLATFORM_IDS: &[&str] = &["minecraft", "neoforge", "forge", "java", "fml", "mcp"];

pub fn is_platform(mod_id: &str) -> bool {
    PLATFORM_IDS.contains(&mod_id.to_ascii_lowercase().as_str())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `modId` required, as written in the descriptor.
    pub mod_id: String,
    /// Maven range, e.g. `[21.1.65,)`. Kept for diagnostics.
    pub version_range: Option<String>,
    pub side: Side,
}

/// Useful content of a mod jar.
#[derive(Debug, Clone, Default)]
pub struct JarInfo {
    /// `modId`s declared by this jar's descriptor. **The mod's identity.**
    ///
    /// It's the only set whose duplication makes NeoForge fail, and
    /// therefore the only one on which two projects can be said to be
    /// redundant.
    pub provides: BTreeSet<String>,
    /// `modId`s brought in by bundled jars (JarJar).
    ///
    /// They satisfy dependencies but define no identity: two mods
    /// legitimately bundle the same library, and NeoForge knows how to
    /// deduplicate them at load time. Confusing them with [`provides`] made
    /// Sodium and Iris look like a duplicate — they share four Fabric
    /// shims — and silently dropped one of the two.
    ///
    /// [`provides`]: JarInfo::provides
    pub bundled: BTreeSet<String>,
    /// Required dependencies, excluding the platform.
    pub requires: Vec<Requirement>,
}

impl JarInfo {
    /// Everything this jar brings, root and bundled combined.
    ///
    /// This is what satisfies a dependency — as opposed to [`provides`],
    /// which says who this mod is.
    ///
    /// [`provides`]: JarInfo::provides
    pub fn provided(&self) -> impl Iterator<Item = &String> {
        self.provides.iter().chain(self.bundled.iter())
    }
}

mod parsing;

pub use parsing::parse_descriptor;
