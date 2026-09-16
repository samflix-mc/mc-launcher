//! Un build précis, et le projet qui porte un modId.

use anyhow::Result;

use crate::Candidate;

use super::api::WebFile;
use super::conversion::to_candidate;
use super::{CurseForgeWeb, WEB};

impl CurseForgeWeb {
    /// Build précis, pour un épinglage du manifeste.
    pub async fn candidate_by_file(
        &self,
        id_or_slug: &str,
        file_id: &str,
    ) -> Result<Option<Candidate>> {
        let Some((project_id, name)) = self.resolve_project(id_or_slug).await? else {
            return Ok(None);
        };
        let Some(file): Option<WebFile> = self
            .get_json(&format!("{WEB}/mods/{project_id}/files/{file_id}"))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(to_candidate(project_id, id_or_slug, &name, file)))
    }

    /// Cherche le projet portant un `modId`, la recherche par mot-clé étant
    /// fermée. Ne fonctionne donc que lorsque le `modId` est aussi le slug.
    pub async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        self.candidates(mod_id, mc, loader).await
    }
}
