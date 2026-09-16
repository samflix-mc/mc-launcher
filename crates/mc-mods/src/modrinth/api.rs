//! Les formes que l'API rend, telles qu'elle les rend.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Project {
    pub(super) id: String,
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) client_side: String,
    pub(super) server_side: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiVersion {
    pub(super) id: String,
    pub(super) project_id: String,
    pub(super) name: String,
    pub(super) version_number: String,
    pub(super) version_type: String,
    pub(super) date_published: String,
    pub(super) files: Vec<ApiFile>,
    pub(super) dependencies: Vec<ApiDependency>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiFile {
    pub(super) url: String,
    pub(super) filename: String,
    pub(super) primary: bool,
    pub(super) size: u64,
    pub(super) hashes: ApiHashes,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiHashes {
    pub(super) sha1: Option<String>,
    pub(super) sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiDependency {
    pub(super) project_id: Option<String>,
    pub(super) version_id: Option<String>,
    pub(super) dependency_type: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchResponse {
    pub(super) hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchHit {
    pub(super) slug: String,
}
