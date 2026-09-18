//! What's asked of Modrinth, and how its refusals are handled.

use anyhow::{Context, Result};

use crate::Candidate;

use super::Modrinth;
use super::api::{ApiVersion, Project};
use super::conversion::to_candidate;

impl Modrinth {
    pub(super) async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .get(url)
            .query(query)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        // A project missing from Modrinth is a normal case: it's what
        // triggers the fallback to CurseForge, not an error to surface.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = response
            .error_for_status()
            .with_context(|| format!("GET {url}"))?;
        Ok(Some(
            response
                .json()
                .await
                .with_context(|| format!("response from {url}"))?,
        ))
    }

    /// Versions published by a project, compatible with `mc` and `loader`.
    ///
    /// `id_or_slug` accepts either the short id (`u6dRKJwZ`) or the slug
    /// (`jei`) indifferently: Modrinth resolves both on the same route,
    /// which makes it possible to treat a dependency — designated by id —
    /// the same as a manifest request, designated by slug.
    pub async fn candidates(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let Some(project): Option<Project> = self
            .get_json(&self.url(&format!("/project/{id_or_slug}")), &[])
            .await?
        else {
            return Ok(Vec::new());
        };

        let query = [
            ("loaders", format!("[\"{loader}\"]")),
            ("game_versions", format!("[\"{mc}\"]")),
        ];
        let versions: Vec<ApiVersion> = self
            .get_json(
                &self.url(&format!("/project/{}/version", project.id)),
                &query,
            )
            .await?
            .unwrap_or_default();

        Ok(versions
            .into_iter()
            .filter_map(|v| to_candidate(&project, v))
            .collect())
    }
}
