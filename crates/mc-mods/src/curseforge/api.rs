//! Les formes que l'API de CurseForge rend.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    pub(crate) data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiMod {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) links: Links,
    #[serde(default)]
    pub(crate) allow_mod_distribution: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Links {
    pub(crate) website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiFile {
    pub(crate) id: u32,
    pub(crate) mod_id: u32,
    pub(crate) display_name: String,
    pub(crate) file_name: String,
    pub(crate) release_type: u32,
    pub(crate) file_date: String,
    pub(crate) download_url: Option<String>,
    pub(crate) file_length: u64,
    #[serde(default)]
    pub(crate) hashes: Vec<ApiHash>,
    #[serde(default)]
    pub(crate) dependencies: Vec<ApiDependency>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiHash {
    pub(crate) value: String,
    pub(crate) algo: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiDependency {
    pub(crate) mod_id: u32,
    pub(crate) relation_type: u32,
}
