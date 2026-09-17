//! Ce que les routes du site rendent.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Page<T> {
    pub(crate) data: Vec<T>,
    #[serde(default)]
    pub(crate) pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Pagination {
    #[serde(rename = "totalCount")]
    pub(crate) total_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebFile {
    pub(crate) id: u64,
    pub(crate) file_name: String,
    pub(crate) display_name: String,
    pub(crate) file_length: u64,
    pub(crate) release_type: u32,
    pub(crate) date_created: String,
    #[serde(default)]
    pub(crate) game_versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebDependency {
    pub(crate) id: u32,
    pub(crate) slug: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Widget {
    pub(crate) id: u32,
    pub(crate) title: String,
}
