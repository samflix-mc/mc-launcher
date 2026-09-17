//! L'API qui publie les Temurin, et ce qu'on lui demande.

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

/// Racine de l'API qui publie les binaires Temurin.
pub(crate) const API: &str = "https://api.adoptium.net/v3";

/// L'URL du dernier binaire publié pour une plateforme et un type d'image.
///
/// La racine est un argument : c'est ce qui permet d'éprouver l'installation
/// complète — téléchargement, vérification d'empreinte, extraction, contrôle du
/// binaire posé — sans sortir sur le réseau ni dépendre de la disponibilité
/// d'Adoptium.
pub(crate) fn url_assets(base: &str, major: u32, os: &str, arch: &str, image: &str) -> String {
    format!(
        "{base}/assets/latest/{major}/hotspot\
         ?architecture={arch}&image_type={image}&os={os}&vendor=eclipse"
    )
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

#[cfg(test)]
#[path = "adoptium.test.rs"]
mod tests;
