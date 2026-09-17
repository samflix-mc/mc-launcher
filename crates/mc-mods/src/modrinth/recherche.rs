//! Retrouver un build précis, ou le projet qui porte un modId.

use anyhow::Result;

use crate::Candidate;

use super::Modrinth;
use super::api::{ApiVersion, Project, SearchResponse};
use super::conversion::to_candidate;

impl Modrinth {
    /// Version précise, pour un build épinglé dans le manifeste.
    pub async fn candidate_by_version(&self, version_id: &str) -> Result<Option<Candidate>> {
        let Some(version): Option<ApiVersion> = self
            .get_json(&self.url(&format!("/version/{version_id}")), &[])
            .await?
        else {
            return Ok(None);
        };
        let Some(project): Option<Project> = self
            .get_json(&self.url(&format!("/project/{}", version.project_id)), &[])
            .await?
        else {
            return Ok(None);
        };
        Ok(to_candidate(&project, version))
    }

    /// Cherche le projet qui porte un `modId` donné.
    ///
    /// Sert au rattrapage des dépendances qu'aucune API ne déclare : le jar dit
    /// « il me faut `bookshelf` », et il faut retrouver le projet
    /// correspondant. Le slug coïncide souvent avec le `modId`, mais pas
    /// toujours — `bookshelf` est publié sous `bookshelf-lib` — d'où la
    /// recherche en second recours.
    pub async fn find_by_mod_id(
        &self,
        mod_id: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let direct = self.candidates(mod_id, mc, loader).await?;
        if !direct.is_empty() {
            return Ok(direct);
        }

        let facets =
            format!("[[\"project_type:mod\"],[\"categories:{loader}\"],[\"versions:{mc}\"]]");
        let query = [
            ("query", mod_id.to_string()),
            ("facets", facets),
            ("limit", "5".to_string()),
        ];
        let Some(found): Option<SearchResponse> =
            self.get_json(&self.url("/search"), &query).await?
        else {
            return Ok(Vec::new());
        };

        // Les résultats sont classés par pertinence ; on s'arrête au premier
        // projet qui a effectivement une version compatible.
        for hit in found.hits {
            let candidates = self.candidates(&hit.slug, mc, loader).await?;
            if !candidates.is_empty() {
                return Ok(candidates);
            }
        }
        Ok(Vec::new())
    }
}
