//! Le client HTTP, ses réessais, et ce qu'il écrit sur le disque.

pub(crate) mod fichier;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use std::time::Duration;

use crate::progression::{Avancement, Observateur};

/// Client HTTP partagé par tout le launcher.
pub struct Downloader {
    client: reqwest::Client,
    retries: u32,
    observateur: Option<Observateur>,
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
        Ok(Self {
            client,
            retries: 3,
            observateur: None,
        })
    }

    /// Branche un observateur de progression.
    ///
    /// Séparé de [`Downloader::new`] pour que la ligne de commande, qui n'en a
    /// pas besoin, n'ait rien à passer. Sans observateur, tout ce qui suit se
    /// réduit à un test de `Option` par morceau reçu.
    #[must_use]
    pub fn observe(mut self, observateur: Observateur) -> Self {
        self.observateur = Some(observateur);
        self
    }

    /// Fait savoir où en est le téléchargement.
    ///
    /// Publique parce que les lots ne se comptent pas ici : seul l'appelant qui
    /// a la liste des assets ou des mods sait combien de fichiers et d'octets
    /// il s'apprête à demander, et c'est cette annonce qui rend un temps
    /// restant calculable.
    pub fn signaler(&self, avancement: Avancement<'_>) {
        if let Some(observateur) = &self.observateur {
            observateur(avancement);
        }
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

    /// Une tentative, lue morceau par morceau.
    ///
    /// `response.bytes()` rendrait le corps entier d'un coup, ce qui est plus
    /// court à écrire et ne laisse rien voir : aucun octet intermédiaire n'est
    /// observable, donc ni débit ni temps restant. Le flux coûte une boucle et
    /// rend l'attente lisible.
    ///
    /// Ce qu'une tentative ratée avait fait compter est retiré avant de rendre
    /// la main — le réessai le recomptera.
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

        // La taille annoncée évite de réallouer le tampon une dizaine de fois
        // sur un jar de cinquante mégaoctets.
        let mut recus = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
        let mut flux = response.bytes_stream();
        while let Some(morceau) = flux.next().await {
            let morceau = match morceau {
                Ok(morceau) => morceau,
                Err(erreur) => {
                    self.signaler(Avancement::Perdus(recus.len() as u64));
                    return Err(erreur.into());
                }
            };
            self.signaler(Avancement::Recus(morceau.len() as u64));
            recus.extend_from_slice(&morceau);
        }
        Ok(recus)
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
