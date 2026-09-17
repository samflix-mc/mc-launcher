//! Le client HTTP, ses réessais, et ce qu'il écrit sur le disque.

pub(crate) mod fichier;

use anyhow::{Context, Result, bail};
use std::time::Duration;

/// Client HTTP partagé par tout le launcher.
pub struct Downloader {
    client: reqwest::Client,
    retries: u32,
}

impl Downloader {
    /// Les deux API publiques utilisées exigent un `User-Agent` identifiant :
    /// Modrinth documente `projet/version (contact)` et applique une limite de
    /// débit par agent, CurseForge journalise l'agent avec la clé.
    pub fn new(user_agent: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .context("construction du client HTTP")?;
        Ok(Self { client, retries: 3 })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Corps de la réponse, avec réessais sur erreur réseau ou 5xx.
    pub async fn bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut last = None;
        for attempt in 0..self.retries {
            tokio::time::sleep(palier(attempt)).await;
            match self.try_bytes(url).await {
                Ok(b) => {
                    tracing::trace!(url, octets = b.len(), tentative = attempt + 1, "GET");
                    return Ok(b);
                }
                Err(e) => {
                    // Un réessai qui finit par réussir ne remonte nulle part
                    // ailleurs : c'est pourtant le premier signe d'une source
                    // qui se dégrade.
                    tracing::warn!(url, tentative = attempt + 1, erreur = %e, "échec, nouvel essai");
                    last = Some(e);
                }
            }
        }
        Err(last.expect("au moins une tentative")).with_context(|| format!("GET {url}"))
    }

    async fn try_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!(
                "HTTP {status} : {}",
                body.chars().take(300).collect::<String>()
            );
        }
        Ok(response.bytes().await?.to_vec())
    }
}

/// Attente avant la tentative numéro `attempt`, la première étant la zéro.
///
/// Palier court et croissant : les 5xx d'un CDN passent en quelques secondes,
/// et un modpack fait des milliers de requêtes — attendre une seconde à chacune
/// coûterait plus cher que les erreurs qu'on évite. La première tentative
/// n'attend pas, ce que dit ici le produit par zéro plutôt qu'une condition :
/// une durée nulle se vérifie, une branche sautée ne se voit pas.
fn palier(attempt: u32) -> Duration {
    Duration::from_millis(400 * u64::from(attempt))
}

#[cfg(test)]
#[path = "telechargement.test.rs"]
mod tests;
