//! What the site's routes render.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Page<T> {
    pub(crate) data: Vec<T>,
    #[serde(default)]
    pub(crate) pagination: Option<Pagination>,
}

/// A response that carries only a single object.
///
/// The site's routes wrap **everything** in `data`, the list just like the
/// unit. Reading a single object without its envelope produces a
/// deserialization failure — and, because the failure was swallowed, a
/// pinned build that exists and gets declared missing.
#[derive(Debug, Deserialize)]
pub(crate) struct Single<T> {
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
