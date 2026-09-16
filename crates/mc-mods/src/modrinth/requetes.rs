//! Ce qu'on demande à Modrinth, et comment on encaisse ses refus.

use anyhow::{Context, Result};

use crate::Candidate;

use super::api::{ApiVersion, Project};
use super::conversion::to_candidate;
use super::{Modrinth, API};

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

        // Un projet absent de Modrinth est un cas nominal : c'est ce qui
        // déclenche le repli sur CurseForge, pas une erreur à remonter.
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
                .with_context(|| format!("réponse de {url}"))?,
        ))
    }

    /// Versions publiées d'un projet, compatibles avec `mc` et `loader`.
    ///
    /// `id_or_slug` accepte indifféremment l'identifiant court (`u6dRKJwZ`) ou
    /// le slug (`jei`) : Modrinth résout les deux sur la même route, ce qui
    /// permet de traiter une dépendance — désignée par identifiant — comme une
    /// demande du manifeste, désignée par slug.
    pub async fn candidates(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let Some(project): Option<Project> = self
            .get_json(&format!("{API}/project/{id_or_slug}"), &[])
            .await?
        else {
            return Ok(Vec::new());
        };

        let query = [
            ("loaders", format!("[\"{loader}\"]")),
            ("game_versions", format!("[\"{mc}\"]")),
        ];
        let versions: Vec<ApiVersion> = self
            .get_json(&format!("{API}/project/{}/version", project.id), &query)
            .await?
            .unwrap_or_default();

        Ok(versions
            .into_iter()
            .filter_map(|v| to_candidate(&project, v))
            .collect())
    }

}
