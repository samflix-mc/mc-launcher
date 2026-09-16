//! L'API qui publie les Temurin, et ce qu'on lui demande.

use anyhow::{bail, Result};
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

/// Couple `(os, architecture)` au vocabulaire d'Adoptium.
pub(crate) fn platform() -> Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        "windows" => "windows",
        other => bail!("système {other} non couvert par les binaires Temurin"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => bail!("architecture {other} non couverte par les binaires Temurin"),
    };
    Ok((os, arch))
}
