//! Chaîne d'authentification d'un launcher Minecraft tiers.
//!
//! Microsoft (device code) → Xbox Live → XSTS → Minecraft → profil.
//!
//! Le *device code flow* évite d'avoir à gérer une URI de redirection et un
//! serveur HTTP local : l'utilisateur ouvre une URL et saisit un code. C'est le
//! flux le plus simple pour une application de bureau, et il est explicitement
//! supporté pour l'API Minecraft.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::time::Duration;

const TENANT: &str = "consumers";
const SCOPE: &str = "XboxLive.signin offline_access";

/// Étape de la chaîne, pour situer une erreur sans lire la trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    DeviceCode,
    Token,
    XboxLive,
    Xsts,
    Minecraft,
    Entitlements,
    Profile,
}

#[derive(Debug)]
pub struct StepError {
    pub step: Step,
    pub status: u16,
    pub body: String,
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} a répondu HTTP {} : {}",
            self.step, self.status, self.body
        )
    }
}

impl std::error::Error for StepError {}

#[derive(Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftToken {
    access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

pub struct Session {
    pub minecraft_token: String,
    pub refresh_token: Option<String>,
    pub profile: Profile,
}

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
    async fn poll_token(&self, dc: &DeviceCode) -> Result<TokenResponse> {
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

    async fn xbox_live(&self, ms_token: &str) -> Result<(String, String)> {
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

    async fn xsts(&self, xbl_token: &str) -> Result<(String, String)> {
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

    async fn minecraft_token(&self, uhs: &str, xsts_token: &str) -> Result<String> {
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
