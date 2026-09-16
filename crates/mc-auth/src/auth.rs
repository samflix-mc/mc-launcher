//! Le client de la chaîne, et ce qu'il fait d'une réponse.

mod appareil;
mod jeu;
mod xbox;

use anyhow::{bail, Result};
use std::time::Duration;

use crate::etape::{Step, StepError};

/// Le client HTTP de la chaîne, et l'identifiant d'application qu'il présente.
pub struct Auth {
    http: reqwest::Client,
    client_id: String,
}

impl Auth {
    pub fn new(client_id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("mc-auth/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(30))
                .build()?,
            client_id: client_id.into(),
        })
    }

    async fn check(&self, step: Step, resp: reqwest::Response) -> Result<String> {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if !(200..300).contains(&status) {
            bail!(StepError { step, status, body });
        }
        Ok(body)
    }
}
