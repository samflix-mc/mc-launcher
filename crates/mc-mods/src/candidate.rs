//! A published version, as the resolver handles it.

use crate::{Channel, Origin, Side};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredDep {
    /// Project ID **in the parent's source**.
    pub project_id: String,
    /// Exact version required, when the source gives one. Modrinth
    /// sometimes does; honoring it avoids installing a version newer than
    /// another dependency forbids.
    pub version_id: Option<String>,
}

/// A published version, candidate for installation.
///
/// Type shared by both sources: the resolver doesn't know where what it's
/// handling comes from, which avoids duplicating its logic per backend.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub origin: Origin,
    pub project_id: String,
    pub slug: String,
    /// Human-readable project name, e.g. "Just Enough Items".
    pub name: String,
    /// Version ID — `version_id` at Modrinth, `fileId` at CurseForge. This is
    /// what gets pinned in the lockfile.
    pub version_id: String,
    pub version_number: String,
    pub display_name: String,
    pub channel: Channel,
    pub file_name: String,
    pub url: String,
    pub sha1: Option<String>,
    /// Published by Modrinth alongside the SHA-1, or computed by us when the
    /// source publishes nothing.
    pub sha512: Option<String>,
    pub size: u64,
    /// ISO 8601 date, used to break ties between two compatible versions.
    pub published: String,
    pub project_side: Side,
    pub declared_deps: Vec<DeclaredDep>,
    pub page_url: Option<String>,
    /// `false` when the author has disabled third-party download.
    pub redistributable: bool,
}

impl Candidate {
    /// The strongest digest we have for this file.
    ///
    /// Modrinth publishes a SHA-512 alongside the SHA-1: preferring it takes
    /// the SHA-1 out of the verification path for most of a pack.
    /// CurseForge only gives SHA-1 or MD5, and that's all there is to check
    /// the file it serves against — hence the fallback.
    pub fn checksum(&self) -> Option<mc_dl::Checksum> {
        self.sha512
            .clone()
            .map(mc_dl::Checksum::Sha512)
            .or_else(|| self.sha1.clone().map(mc_dl::Checksum::Sha1))
    }
}

#[cfg(test)]
#[path = "candidate.test.rs"]
mod tests;
