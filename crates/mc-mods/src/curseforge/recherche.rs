//! Retrouver un projet par le `modId` qu'un jar exige.

use anyhow::{Context, Result, bail};

use crate::Candidate;

use super::api::{ApiMod, Envelope};
use super::cle::{KEY_REFUSED, config_key_path};
use super::conversion::loader_type;
use super::{CLASS_MODS, CurseForge, GAME_MINECRAFT};

impl CurseForge {
    pub(super) async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .post(url)
            .header("x-api-key", &self.key)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => return Ok(None),
            // Même traitement que sur `get_json`, et pour la même raison : le
            // registre bascule sur l'accès sans clé quand il reconnaît ce
            // refus. Sans ce cas, un build épinglé chez CurseForge arrêtait
            // l'installation dès que la clé était révoquée — alors que le mode
            // sans clé savait le servir.
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
            .with_context(|| format!("POST {url}"))?;
        Ok(Some(response.json().await?))
    }

    /// Cherche le projet portant un `modId`, pour le rattrapage des
    /// dépendances qu'aucune API ne déclare.
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

        let query = [
            ("gameId", GAME_MINECRAFT.to_string()),
            ("classId", CLASS_MODS.to_string()),
            ("searchFilter", mod_id.to_string()),
            ("gameVersion", mc.to_string()),
            ("modLoaderType", loader_type(loader).to_string()),
            ("pageSize", "5".to_string()),
        ];
        let found: Option<Envelope<Vec<ApiMod>>> =
            self.get_json(&self.url("/mods/search"), &query).await?;
        let Some(hits) = found else {
            return Ok(Vec::new());
        };

        for hit in hits.data {
            let candidates = self.candidates(&hit.id.to_string(), mc, loader).await?;
            if !candidates.is_empty() {
                return Ok(candidates);
            }
        }
        Ok(Vec::new())
    }
}
