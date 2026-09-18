//! CurseForge, through its site's public API.
//!
//! The Core API — the one that requires a registration key — is no longer
//! queried. What's left is the API the site serves to its own pages,
//! completed by cfwidget to recover a project id from a slug.
//!
//! What this choice costs is spelled out in `docs/mods.md`, and it's not
//! nothing: no published digest, no keyword search, and no more way to know
//! `allowModDistribution`.

use anyhow::Result;

use crate::Candidate;
use crate::resolve::Registry;

impl Registry {
    /// Candidates for a project.
    pub(crate) async fn curseforge_any(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        self.curseforge_web.candidates(id_or_slug, mc, loader).await
    }

    /// Looks up a project by the `modId` a jar declares.
    ///
    /// Modrinth first: its search is open and returns the right project even
    /// when the `modId` doesn't look like the slug. CurseForge next, where
    /// keyword search is closed — only a `modId` that's also the project's
    /// slug can succeed.
    pub(crate) async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let found = self.modrinth.find_by_mod_id(mod_id, mc, loader).await?;
        if !found.is_empty() {
            return Ok(found);
        }
        // A failure isn't an absence of result, but here it amounts to the
        // same thing: it's the last source, and the catch-up loop must be
        // able to conclude that this `modId` has no project rather than
        // stopping everything.
        self.curseforge_web
            .find_by_mod_id(mod_id, mc, loader)
            .await
            .or_else(|_| Ok(Vec::new()))
    }

    /// CurseForge build pinned by the lockfile.
    pub(crate) async fn curseforge_file(
        &self,
        id_or_slug: &str,
        file: &str,
    ) -> Result<Option<Candidate>> {
        self.curseforge_web
            .candidate_by_file(id_or_slug, file)
            .await
    }
}

#[cfg(test)]
#[path = "curseforge.test.rs"]
mod tests;
