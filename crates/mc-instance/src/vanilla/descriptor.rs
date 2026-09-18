//! The shapes Mojang publishes in its descriptors.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) versions: Vec<ManifestVersion>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ManifestVersion {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) sha1: String,
}

#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LibraryDownloads {
    pub(crate) artifact: Option<Artifact>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Library {
    pub(crate) name: String,
    pub(crate) downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub(crate) rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Rule {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) os: Option<OsCondition>,
    /// Flags the launcher does or doesn't enable — demo, forced resolution,
    /// Quick Play. Absent from library rules, present on argument ones,
    /// hence the same type for both.
    #[serde(default)]
    pub(crate) features: Option<std::collections::BTreeMap<String, bool>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OsCondition {
    pub(crate) name: Option<String>,
    pub(crate) arch: Option<String>,
}

/// Flags active for this run, compared against the rules' `features`.
pub(crate) type Features = std::collections::BTreeSet<String>;

#[derive(Debug, Deserialize)]
pub(crate) struct AssetIndexRef {
    pub(crate) id: String,
    pub(crate) sha1: String,
    pub(crate) url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Downloads {
    pub(crate) client: Artifact,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VersionJson {
    pub(crate) id: String,
    pub(crate) main_class: String,
    pub(crate) asset_index: AssetIndexRef,
    pub(crate) downloads: Downloads,
    pub(crate) libraries: Vec<Library>,
    pub(crate) java_version: Option<JavaVersion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JavaVersion {
    pub(crate) major_version: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssetIndex {
    pub(crate) objects: std::collections::BTreeMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssetObject {
    pub(crate) hash: String,
    pub(crate) size: u64,
}
