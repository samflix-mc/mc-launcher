//! Le chargeur et les mods, tels que le manifeste les demande.

use anyhow::{Context, Result};
use mc_mods::{Channel, Origin, Request, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loader {
    #[serde(rename = "type")]
    pub kind: String,
    /// Version exacte, ou `latest` pour la dernière publiée de la série
    /// correspondant à la version du jeu.
    pub version: String,
}

impl Loader {
    pub fn is_latest(&self) -> bool {
        self.version.eq_ignore_ascii_case("latest")
    }
}

/// Un mod demandé.
///
/// Seul `slug` est obligatoire. Les autres champs servent à sortir du
/// comportement par défaut, et chacun consigne une décision :
/// `file` fige un build, `side` contredit ce que la plateforme annonce,
/// `channel` autorise une préversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Origin>,
    /// Identifiant du build épinglé — `version_id` Modrinth, `fileId`
    /// CurseForge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Numéro de version publié, plus lisible qu'un identifiant.
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
                    .with_context(|| format!("côté inconnu pour {} : « {text} »", self.slug))?,
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
            // Le manifeste ne porte pas d'empreinte : elle vient du verrou,
            // qui est justement ce que le manifeste ne veut pas répéter.
            expected_sha1: None,
            expected_sha512: None,
        })
    }
}
