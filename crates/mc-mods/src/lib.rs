//! Resolution and download of a pack's mods.
//!
//! The manifest names a few mods; the `mods` folder always holds more. The
//! gap is dependencies — and this crate's job is to fill it without
//! intervention.
//!
//! Three sources of information are cross-checked, in increasing order of
//! reliability:
//!
//! 1. **what the manifest asks for** — possibly a pinned build;
//! 2. **what the API declares** — the dependencies the author entered at
//!    publish time, often incomplete;
//! 3. **what the jar requires** — `META-INF/neoforge.mods.toml`, the only
//!    source the game actually reads.
//!
//! Mods are looked up across two sources, from the most reliable to the
//! least contractual: [`modrinth`], then [`curseforge_web`] — CurseForge's
//! public website API, keyless, with the limits its module details.
//!
//! The CurseForge Core API, the one that needs a registration key, has been
//! removed: the launcher no longer depends on it, and nobody has to supply a
//! key to install a pack.
//!
//! The third point is the one that decides: after download, each jar is
//! opened, its mandatory `modId`s compared against what the pack supplies,
//! and any gap triggers another resolution pass. It stops once nothing is
//! missing anymore — which is exactly the condition NeoForge will check at
//! startup.
mod candidate;
mod channel;
mod origin;

pub mod curseforge_web;
pub mod jar;
pub mod modrinth;
pub mod resolve;

pub use candidate::{Candidate, DeclaredDep};
pub use channel::Channel;
pub use jar::Side;
pub use origin::Origin;
pub use resolve::{
    Installed, Options, Plan, Progress, Reason, Registry, Request, resolve, resolve_with,
};

#[cfg(test)]
mod fixtures;
