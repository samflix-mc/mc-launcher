//! From the manifest to the `mods` folder: resolution, download, catch-up.
//!
//! The cycle is deliberately iterative rather than recursive over metadata
//! alone:
//!
//! ```text
//!   manifest requests
//!        ↓  resolution (Modrinth, then CurseForge)
//!   candidates + declared dependencies
//!        ↓  verified download
//!   jars on disk
//!        ↓  reading neoforge.mods.toml
//!   modId provided / modId required
//!        ↓  non-empty gap? → another pass
//!   stable plan
//! ```
//!
//! The last pass is the one that matters: it catches the dependencies no
//! API declares. That's the common case — an author who adds a library
//! between two versions doesn't go back and edit the publish page — and
//! it's exactly what crashes a client at startup with a "Missing or
//! unsupported mods" screen.

mod catchup;
mod choice;
mod deployment;
mod download;
mod duplicates;
#[cfg(test)]
mod fixtures;
mod inspection;
mod options;
mod plan;
mod queue;
mod reason;
mod registry;
mod request;
mod resolution_loop;

pub use deployment::{Deployed, deploy};
pub use options::Options;
pub use plan::{Installed, Plan, Unresolved};
pub use reason::Reason;
pub use registry::{Progress, Registry};
pub use request::Request;
pub use resolution_loop::{resolve, resolve_with};

/// Number of catch-up passes.
///
/// A chain of implicit dependencies rarely goes past two levels; the bound
/// guards against a loop if two mods claim each other without the search
/// converging.
const MAX_PASSES: usize = 6;

/// Simultaneous downloads. Modrinth rate-limits per agent: beyond a handful
/// of connections, the 429s cost more time than they save.
const PARALLEL_DOWNLOADS: usize = 6;
