//! Ce que les routes du site rendent.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Page<T> {
    pub(crate) data: Vec<T>,
    #[serde(default)]
    pub(crate) pagination: Option<Pagination>,
}

/// Une réponse qui ne porte qu'un objet.
///
/// Les routes du site enveloppent **tout** dans `data`, la liste comme l'unité.
/// Lire un objet unique sans son enveloppe donne une désérialisation qui
/// échoue — et, parce que l'échec était avalé, un build épinglé qui existe et
/// qu'on déclare introuvable.
#[derive(Debug, Deserialize)]
pub(crate) struct Un<T> {
    pub(crate) data: T,
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
