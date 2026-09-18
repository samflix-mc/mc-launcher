//! A precise build, and the project that carries a modId.

use anyhow::Result;

use crate::Candidate;

use super::CurseForgeWeb;
use super::api::{Single, WebFile};
use super::conversion::to_candidate;

impl CurseForgeWeb {
    /// Precise build, for a manifest pin.
    pub async fn candidate_by_file(
        &self,
        id_or_slug: &str,
        file_id: &str,
    ) -> Result<Option<Candidate>> {
        let Some((project_id, name)) = self.resolve_project(id_or_slug).await? else {
            return Ok(None);
        };
        // `Single<WebFile>` and not `WebFile`: the route wraps the object in
        // `data`, like the lists. Reading it without the envelope made
        // deserialization fail, and a pinned build that was genuinely there
        // was declared missing.
        let Some(file): Option<Single<WebFile>> = self
            .get_json(&format!("{}/mods/{project_id}/files/{file_id}", self.web))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(to_candidate(
            &self.web, project_id, id_or_slug, &name, file.data,
        )))
    }

    /// Look up the project carrying a `modId`, since keyword search is
    /// closed. Only works when the `modId` is also the slug.
    pub async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        self.candidates(mod_id, mc, loader).await
    }
}

#[cfg(test)]
#[path = "pinned.test.rs"]
mod tests;
