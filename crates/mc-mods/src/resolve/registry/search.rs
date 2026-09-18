//! The candidates a source offers for a request.

use anyhow::{Context, Result};

use crate::resolve::Registry;
use crate::resolve::request::Request;
use crate::{Candidate, Origin};

use super::filter::check_compatible;

impl Registry {
    /// Candidates for a project, from the requested source or in the default
    /// order.
    ///
    /// A **numeric** identifier can only come from CurseForge: offering it to
    /// Modrinth would make a request doomed to fail for every resolved
    /// dependency.
    pub(crate) async fn candidates(
        &self,
        id_or_slug: &str,
        source: Option<Origin>,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let numeric = id_or_slug.parse::<u32>().is_ok();

        if source != Some(Origin::CurseForge) && !numeric {
            let found = self.modrinth.candidates(id_or_slug, mc, loader).await?;
            if !found.is_empty() || source == Some(Origin::Modrinth) {
                return Ok(found);
            }
        }
        self.curseforge_any(id_or_slug, mc, loader).await
    }
    pub(crate) async fn pinned(
        &self,
        request: &Request,
        mc: &str,
        loader: &str,
    ) -> Result<Option<Candidate>> {
        let Some(file) = &request.file else {
            return Ok(None);
        };
        // Pinning is explicit: if the build no longer exists, it's better to
        // stop than to silently fall back to another version — that's
        // exactly what pinning is meant to prevent.
        let numeric = request.slug.parse::<u32>().is_ok();
        let found = match request.source {
            Some(Origin::CurseForge) => self.curseforge_file(&request.slug, file).await?,
            Some(Origin::Modrinth) => self.modrinth.candidate_by_version(file).await?,
            None if numeric => self.curseforge_file(&request.slug, file).await?,
            None => match self.modrinth.candidate_by_version(file).await? {
                Some(found) => Some(found),
                None => self.curseforge_file(&request.slug, file).await?,
            },
        };

        let found =
            found.with_context(|| format!("build {file} pinned for {}: missing", request.slug))?;
        check_compatible(&found, mc, loader, &request.slug)?;
        Ok(Some(found))
    }
}

#[cfg(test)]
#[path = "search.test.rs"]
mod tests;
