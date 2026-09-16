//! Dernières étapes : le jeton Minecraft, la licence, le profil.

use anyhow::{bail, Context, Result};

use crate::reponses::{DeviceCode, MinecraftToken, Profile};
use crate::{Auth, Session, Step, StepError};

impl Auth {
    pub(super) async fn minecraft_token(&self, uhs: &str, xsts_token: &str) -> Result<String> {
        let resp = self
            .http
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
            }))
            .send()
            .await
            .context("appel login_with_xbox")?;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if status == 403 {
            bail!(
                "HTTP 403 sur api.minecraftservices.com.\n\
                 C'est la réponse attendue tant que le Client ID Azure n'a pas été approuvé \
                 par Microsoft. Cette tentative compte comme l'activité exigée avant de \
                 soumettre le formulaire https://aka.ms/mce-reviewappid.\n\
                 Corps : {body}"
            );
        }
        if !(200..300).contains(&status) {
            bail!(StepError {
                step: Step::Minecraft,
                status,
                body
            });
        }
        let r: MinecraftToken =
            serde_json::from_str(&body).context("réponse Minecraft illisible")?;
        Ok(r.access_token)
    }

    /// Vérifie que le compte possède le jeu. Un tableau vide = pas de licence.
    pub async fn owns_game(&self, mc_token: &str) -> Result<bool> {
        let resp = self
            .http
            .get("https://api.minecraftservices.com/entitlements/mcstore")
            .bearer_auth(mc_token)
            .send()
            .await
            .context("appel entitlements")?;
        let body = self.check(Step::Entitlements, resp).await?;
        let v: serde_json::Value =
            serde_json::from_str(&body).context("entitlements illisibles")?;
        Ok(v["items"].as_array().is_some_and(|a| !a.is_empty()))
    }

    pub async fn profile(&self, mc_token: &str) -> Result<Profile> {
        let resp = self
            .http
            .get("https://api.minecraftservices.com/minecraft/profile")
            .bearer_auth(mc_token)
            .send()
            .await
            .context("appel profil")?;
        let body = self.check(Step::Profile, resp).await?;
        serde_json::from_str(&body).context("profil illisible")
    }

    /// Déroule toute la chaîne. `on_code` reçoit le code à afficher à l'utilisateur.
    pub async fn login(&self, on_code: impl FnOnce(&DeviceCode)) -> Result<Session> {
        let dc = self.device_code().await?;
        on_code(&dc);
        let token = self.poll_token(&dc).await?;
        let (xbl, _) = self.xbox_live(&token.access_token).await?;
        let (xsts, uhs) = self.xsts(&xbl).await?;
        let mc = self.minecraft_token(&uhs, &xsts).await?;
        if !self.owns_game(&mc).await? {
            bail!("ce compte ne possède pas Minecraft Java Edition");
        }
        let profile = self.profile(&mc).await?;
        Ok(Session {
            minecraft_token: mc,
            refresh_token: token.refresh_token,
            profile,
        })
    }
}
