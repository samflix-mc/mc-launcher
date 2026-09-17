//! Un build précis, et le projet qui porte un modId.

use anyhow::Result;

use crate::Candidate;

use super::CurseForgeWeb;
use super::api::{Un, WebFile};
use super::conversion::to_candidate;

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
        // `Un<WebFile>` et non `WebFile` : la route enveloppe l'objet dans
        // `data`, comme les listes. Le lire sans enveloppe faisait échouer la
        // désérialisation, et un build épinglé bien présent était déclaré
        // introuvable.
        let Some(file): Option<Un<WebFile>> = self
            .get_json(&format!("{}/mods/{project_id}/files/{file_id}", self.web))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(to_candidate(
            &self.web, project_id, id_or_slug, &name, file.data,
        )))
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

#[cfg(test)]
#[path = "epingle.test.rs"]
mod tests;
