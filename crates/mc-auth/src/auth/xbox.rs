//! Deuxième et troisième étapes : Xbox Live, puis XSTS.

use anyhow::{bail, Context, Result};

use crate::reponses::XboxResponse;
use crate::{Auth, Step, StepError};

impl Auth {
    pub(super) async fn xbox_live(&self, ms_token: &str) -> Result<(String, String)> {
        let resp = self
            .http
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "Properties": {
                    "AuthMethod": "RPS",
                    "SiteName": "user.auth.xboxlive.com",
                    "RpsTicket": format!("d={ms_token}"),
                },
                "RelyingParty": "http://auth.xboxlive.com",
                "TokenType": "JWT",
            }))
            .send()
            .await
            .context("appel Xbox Live")?;
        let body = self.check(Step::XboxLive, resp).await?;
        let r: XboxResponse = serde_json::from_str(&body).context("réponse XBL illisible")?;
        let uhs = r
            .display_claims
            .xui
            .first()
            .context("uhs absent")?
            .uhs
            .clone();
        Ok((r.token, uhs))
    }

    pub(super) async fn xsts(&self, xbl_token: &str) -> Result<(String, String)> {
        let resp = self
            .http
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
                "RelyingParty": "rp://api.minecraftservices.com/",
                "TokenType": "JWT",
            }))
            .send()
            .await
            .context("appel XSTS")?;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if status == 401 {
            let xerr = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v["XErr"].as_u64())
                .unwrap_or(0);
            let explication = match xerr {
                2148916227 => "ce compte Xbox est banni",
                2148916233 => "ce compte Microsoft n'a pas de compte Xbox rattaché",
                2148916235 => "Xbox Live n'est pas disponible dans ce pays",
                2148916236 | 2148916237 => "vérification d'âge requise (Corée du Sud)",
                2148916238 => "compte enfant : il doit être rattaché à une famille",
                _ => "refus XSTS non répertorié",
            };
            bail!("XSTS a refusé ({xerr}) : {explication}");
        }
        if !(200..300).contains(&status) {
            bail!(StepError {
                step: Step::Xsts,
                status,
                body
            });
        }
        let r: XboxResponse = serde_json::from_str(&body).context("réponse XSTS illisible")?;
        let uhs = r
            .display_claims
            .xui
            .first()
            .context("uhs absent")?
            .uhs
            .clone();
        Ok((r.token, uhs))
    }
}
