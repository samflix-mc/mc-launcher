//! The loader and mods, as the manifest requests them.

use anyhow::{Context, Result};
use mc_mods::{Channel, Origin, Request, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loader {
    #[serde(rename = "type")]
    pub kind: String,
    /// Exact version, or `latest` for the most recently published one in
    /// the series matching the game version.
    pub version: String,
}

impl Loader {
    pub fn is_latest(&self) -> bool {
        self.version.eq_ignore_ascii_case("latest")
    }
}

/// A requested mod.
///
/// Only `slug` is required. The other fields exist to step out of the
/// default behavior, and each records a decision: `file` pins a build,
/// `side` contradicts what the platform announces, `channel` allows a
/// pre-release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Origin>,
    /// Identifier of the pinned build — Modrinth's `version_id`, CurseForge's
    /// `fileId`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Published version number, more readable than an identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
}

impl ModEntry {
    pub fn to_request(&self) -> Result<Request> {
        let side = match &self.side {
            Some(text) => Some(
                Side::parse(text)
                    .with_context(|| format!("unknown side for {}: “{text}”", self.slug))?,
            ),
            None => None,
        };
        Ok(Request {
            slug: self.slug.clone(),
            source: self.source,
            file: self.file.clone(),
            version: self.version.clone(),
            side,
            channel: self.channel,
            // The manifest carries no digest: it comes from the lockfile,
            // which is exactly what the manifest doesn't want to repeat.
            expected_sha1: None,
            expected_sha512: None,
        })
    }
}
