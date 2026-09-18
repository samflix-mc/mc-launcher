//! What we request, and where we request it from.

use crate::jar::Side;
use crate::{Channel, Origin};

/// A mod requested by the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Slug, or project identifier if the source is forced.
    pub slug: String,
    /// Forces the source. Without a value: Modrinth, then CurseForge.
    pub source: Option<Origin>,
    /// Pins an exact build — Modrinth `version_id` or CurseForge `fileId`.
    /// This is what makes an install reproducible.
    pub file: Option<String>,
    /// Pins a published version number, more readable than an identifier.
    pub version: Option<String>,
    /// Forces the side, instead of deducing it from the metadata.
    pub side: Option<Side>,
    /// The most unstable channel accepted. `release` by default.
    pub channel: Option<Channel>,
    /// Digest known from elsewhere, typically carried over from the lock.
    ///
    /// Some sources don't publish a digest. The one a first pass computed
    /// and fixed in the lock makes the following ones just as verifiable as
    /// for the other sources.
    pub expected_sha1: Option<String>,
    /// Same, when the lock carries the strong digest.
    pub expected_sha512: Option<String>,
}

impl Request {
    pub fn new(slug: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            source: None,
            file: None,
            version: None,
            side: None,
            channel: None,
            expected_sha1: None,
            expected_sha512: None,
        }
    }
}
