//! Ce qu'on demande, et d'où on le demande.

use crate::jar::Side;
use crate::{Channel, Origin};

/// Un mod demandé par le manifeste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Slug, ou identifiant de projet si la source est imposée.
    pub slug: String,
    /// Force la source. Sans valeur : Modrinth, puis CurseForge.
    pub source: Option<Origin>,
    /// Épingle un build précis — `version_id` Modrinth ou `fileId` CurseForge.
    /// C'est ce qui rend une installation reproductible.
    pub file: Option<String>,
    /// Épingle un numéro de version publié, plus lisible qu'un identifiant.
    pub version: Option<String>,
    /// Impose le côté, au lieu de le déduire des métadonnées.
    pub side: Option<Side>,
    /// Canal le plus instable accepté. `release` par défaut.
    pub channel: Option<Channel>,
    /// Empreinte connue par ailleurs, typiquement reprise du verrou.
    ///
    /// Certaines sources ne publient pas d'empreinte. Celle qu'un premier
    /// passage a calculée et figée dans le verrou rend les suivants aussi
    /// vérifiables que pour les autres sources.
    pub expected_sha1: Option<String>,
    /// Idem, quand le verrou porte l'empreinte forte.
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
