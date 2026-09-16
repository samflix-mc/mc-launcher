//! Ce que chaque service répond, tel qu'il le répond.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct XboxResponse {
    #[serde(rename = "Token")]
    pub(crate) token: String,
    #[serde(rename = "DisplayClaims")]
    pub(crate) display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DisplayClaims {
    pub(crate) xui: Vec<Xui>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Xui {
    pub(crate) uhs: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MinecraftToken {
    pub(crate) access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}
