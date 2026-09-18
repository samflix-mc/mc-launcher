//! Look up a precise build, or the project that carries a modId.

use anyhow::Result;

use crate::Candidate;

use super::Modrinth;
use super::api::{ApiVersion, Project, SearchResponse};
use super::conversion::to_candidate;

impl Modrinth {
    /// Precise version, for a build pinned in the manifest.
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

    /// Look up the project that carries a given `modId`.
    ///
    /// Used for catching up on dependencies that no API declares: the jar
    /// says "I need `bookshelf`", and the matching project has to be found.
    /// The slug often matches the `modId`, but not always — `bookshelf` is
    /// published under `bookshelf-lib` — hence the fallback search.
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

        // Results are ranked by relevance; the first project that actually
        // has a compatible version wins.
        for hit in found.hits {
            let candidates = self.candidates(&hit.slug, mc, loader).await?;
            if !candidates.is_empty() {
                return Ok(candidates);
            }
        }
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[path = "search.test.rs"]
mod tests;
