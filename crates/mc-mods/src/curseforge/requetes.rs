//! Ce qu'on demande à CurseForge.

use anyhow::{bail, Context, Result};

use super::api::{ApiMod, Envelope};
use super::cle::KEY_REFUSED;
use super::cle::config_key_path;
use super::{CurseForge, API, CLASS_MODS, GAME_MINECRAFT};

impl CurseForge {
    pub(super) async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .get(url)
            .header("x-api-key", &self.key)
            .query(query)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => return Ok(None),
            reqwest::StatusCode::FORBIDDEN | reqwest::StatusCode::UNAUTHORIZED => {
                bail!(
                    "{KEY_REFUSED} (HTTP {}). Vérifier CURSEFORGE_API_KEY ou {} — \
                     une clé de la Core API commence par « $2a$10$ », ce n'est pas un UUID",
                    response.status(),
                    config_key_path().display()
                );
            }
            _ => {}
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

    pub(super) async fn project_by_slug(&self, slug: &str) -> Result<Option<ApiMod>> {
        let query = [
            ("gameId", GAME_MINECRAFT.to_string()),
            ("classId", CLASS_MODS.to_string()),
            ("slug", slug.to_string()),
        ];
        let found: Option<Envelope<Vec<ApiMod>>> =
            self.get_json(&format!("{API}/mods/search"), &query).await?;
        Ok(found.and_then(|e| e.data.into_iter().next()))
    }

    pub(super) async fn project_by_id(&self, id: u32) -> Result<Option<ApiMod>> {
        let found: Option<Envelope<ApiMod>> =
            self.get_json(&format!("{API}/mods/{id}"), &[]).await?;
        Ok(found.map(|e| e.data))
    }
}
