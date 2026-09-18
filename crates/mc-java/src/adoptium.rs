//! The API that publishes Temurin, and what we ask it for.

use anyhow::{Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Asset {
    pub(crate) binary: Binary,
    pub(crate) release_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Binary {
    pub(crate) package: Package,
    pub(crate) image_type: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Package {
    pub(crate) link: String,
    pub(crate) name: String,
    pub(crate) checksum: String,
}

/// Root of the API that publishes Temurin binaries.
pub(crate) const API: &str = "https://api.adoptium.net/v3";

/// The URL of the latest binary published for a platform and an image type.
///
/// The root is an argument: it's what allows the full installation to be
/// exercised — download, digest verification, extraction, check of the
/// placed binary — without going out on the network or depending on
/// Adoptium's availability.
pub(crate) fn url_assets(base: &str, major: u32, os: &str, arch: &str, image: &str) -> String {
    format!(
        "{base}/assets/latest/{major}/hotspot\
         ?architecture={arch}&image_type={image}&os={os}&vendor=eclipse"
    )
}

/// Maps the `(os, architecture)` pair to Adoptium's vocabulary.
pub(crate) fn platform() -> Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        "windows" => "windows",
        other => bail!("system {other} not covered by Temurin binaries"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => bail!("architecture {other} not covered by Temurin binaries"),
    };
    Ok((os, arch))
}

#[cfg(test)]
#[path = "adoptium.test.rs"]
mod tests;
