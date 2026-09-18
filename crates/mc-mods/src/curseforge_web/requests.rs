//! A project's visible builds, and its declared dependencies.

use anyhow::Result;

use crate::{Candidate, DeclaredDep};

use super::api::{Page, WebDependency, WebFile};
use super::conversion::{compatible, to_candidate};
use super::{CurseForgeWeb, PAGE_SIZE};

impl CurseForgeWeb {
    /// Compatible versions published by a project.
    pub async fn candidates(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let Some((project_id, name)) = self.resolve_project(id_or_slug).await? else {
            return Ok(Vec::new());
        };

        let Some(page): Option<Page<WebFile>> = self
            .get_json(&format!(
                "{}/mods/{project_id}/files?pageSize={PAGE_SIZE}&removeAlphas=false",
                self.web
            ))
            .await?
        else {
            return Ok(Vec::new());
        };

        let total = page.pagination.as_ref().map(|p| p.total_count).unwrap_or(0);
        let rendered = page.data.len();

        let slug = if id_or_slug.parse::<u32>().is_ok() {
            format!("{project_id}")
        } else {
            id_or_slug.to_string()
        };

        let found: Vec<Candidate> = page
            .data
            .into_iter()
            .filter(|f| compatible(&f.game_versions, mc, loader))
            .map(|f| to_candidate(&self.web, project_id, &slug, &name, f))
            .collect();

        // Nothing found even though the project publishes far more than
        // what's visible: staying silent would be misleading, the version
        // may well exist outside the window.
        if found.is_empty() && total > rendered {
            anyhow::bail!(
                "{slug}: no {mc}/{loader} version among the {rendered} most recent files, but \
                 the project has {total}. CurseForge doesn't show beyond that. Pin the desired \
                 build with \"file\" in the manifest — its id is in the file page's address."
            );
        }

        let mut found = found;
        if let Some(deps) = self.dependencies(project_id).await? {
            // The site's dependencies are declared per project, not per
            // file: they hold for every version.
            for candidate in &mut found {
                candidate.declared_deps = deps.clone();
            }
        }
        Ok(found)
    }

    /// Required dependencies declared at the project level.
    pub(super) async fn dependencies(&self, project_id: u32) -> Result<Option<Vec<DeclaredDep>>> {
        let Some(page): Option<Page<WebDependency>> = self
            .get_json(&format!(
                "{}/mods/{project_id}/dependencies?pageSize=20",
                self.web
            ))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(
            page.data
                .into_iter()
                .filter(|d| d.kind == "RequiredDependency")
                .map(|d| DeclaredDep {
                    // The slug is preferred over the numeric id: it makes it
                    // possible to find the project on Modrinth, which
                    // publishes digests and the client/server split.
                    project_id: if d.slug.is_empty() {
                        d.id.to_string()
                    } else {
                        d.slug
                    },
                    version_id: None,
                })
                .collect(),
        ))
    }
}

#[cfg(test)]
#[path = "requests.test.rs"]
mod tests;
