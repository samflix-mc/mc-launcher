//! Retrouver l'identifiant numérique d'un projet à partir de son slug.

use anyhow::{Context, Result};

use super::api::Widget;
use super::{CurseForgeWeb, WIDGET};

impl CurseForgeWeb {
    pub(super) async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<Option<T>> {
        let response = self
            .dl
            .client()
            .get(url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        // 403 est la réponse de Cloudflare comme celle d'une route fermée :
        // dans les deux cas, cette source n'a rien à offrir, et l'appelant doit
        // pouvoir continuer sans que tout s'arrête.
        if !response.status().is_success() {
            return Ok(None);
        }
        Ok(response.json().await.ok())
    }

    /// Identifiant de projet à partir d'un slug.
    ///
    /// La recherche du site répond 403 ; cfwidget est la seule voie restante.
    /// Il répond 202 le temps de constituer son cache pour un projet qu'il n'a
    /// jamais vu, d'où la seconde tentative.
    pub(super) async fn project_id(&self, slug: &str) -> Result<Option<(u32, String)>> {
        for attempt in 0..2 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            let response = self
                .dl
                .client()
                .get(format!("{WIDGET}/{slug}"))
                .send()
                .await
                .with_context(|| format!("cfwidget pour {slug}"))?;

            if response.status() == reqwest::StatusCode::ACCEPTED {
                continue;
            }
            if !response.status().is_success() {
                return Ok(None);
            }
            let widget: Widget = match response.json().await {
                Ok(w) => w,
                Err(_) => return Ok(None),
            };
            return Ok(Some((widget.id, widget.title)));
        }
        Ok(None)
    }

    /// Résout un slug ou un identifiant numérique en `(id, nom)`.
    pub(super) async fn resolve_project(&self, id_or_slug: &str) -> Result<Option<(u32, String)>> {
        match id_or_slug.parse::<u32>() {
            // Un identifiant numérique vient d'une dépendance déjà résolue : le
            // nom lisible n'est pas indispensable, et l'économiser évite un
            // appel à un service tiers.
            Ok(id) => Ok(Some((id, format!("projet {id}")))),
            Err(_) => self.project_id(id_or_slug).await,
        }
    }

}
