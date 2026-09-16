//! Première étape : le code que l'utilisateur saisit dans son navigateur.

use anyhow::{bail, Context, Result};

use crate::reponses::{DeviceCode, TokenResponse};
use std::time::Duration;

use crate::{Auth, Step, StepError, SCOPE, TENANT};

impl Auth {
    /// Demande le couple code utilisateur / URL de vérification.
    pub async fn device_code(&self) -> Result<DeviceCode> {
        let url = format!("https://login.microsoftonline.com/{TENANT}/oauth2/v2.0/devicecode");
        let resp = self
            .http
            .post(&url)
            .form(&[("client_id", self.client_id.as_str()), ("scope", SCOPE)])
            .send()
            .await
            .context("appel devicecode")?;
        let body = self.check(Step::DeviceCode, resp).await?;
        serde_json::from_str(&body).context("réponse devicecode illisible")
    }

    /// Attend que l'utilisateur ait validé le code dans son navigateur.
    pub(super) async fn poll_token(&self, dc: &DeviceCode) -> Result<TokenResponse> {
        let url = format!("https://login.microsoftonline.com/{TENANT}/oauth2/v2.0/token");
        let deadline = std::time::Instant::now() + Duration::from_secs(dc.expires_in);
        let mut interval = Duration::from_secs(dc.interval.max(1));
        loop {
            if std::time::Instant::now() > deadline {
                bail!("le code a expiré avant d'être validé");
            }
            tokio::time::sleep(interval).await;
            let resp = self
                .http
                .post(&url)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("client_id", self.client_id.as_str()),
                    ("device_code", dc.device_code.as_str()),
                ])
                .send()
                .await
                .context("appel token")?;
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            if (200..300).contains(&status) {
                return serde_json::from_str(&body).context("réponse token illisible");
            }
            // Les deux seules erreurs qui ne sont pas fatales pendant l'attente.
            let err = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["error"].as_str().map(str::to_owned))
                .unwrap_or_default();
            match err.as_str() {
                "authorization_pending" => {}
                "slow_down" => interval += Duration::from_secs(5),
                _ => bail!(StepError {
                    step: Step::Token,
                    status,
                    body
                }),
            }
        }
    }
}
