//! Ce qu'une ligne de verrou retient d'un mod.

use mc_mods::Origin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedLoader {
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMod {
    pub slug: String,
    pub name: String,
    pub origin: Origin,
    /// Identifiant du projet dans sa source.
    pub project: String,
    /// Identifiant du build. C'est lui qui permet de rejouer l'installation.
    pub file: String,
    pub version: String,
    pub file_name: String,
    /// URL de téléchargement directe, telle que la source l'a donnée.
    ///
    /// Elle rend le verrou exploitable par autre chose que le launcher : la CI
    /// de mc-content vérifie qu'elle répond, et un serveur peut installer le
    /// jar sans rien savoir de Modrinth. Tolérée absente, pour lire les verrous
    /// écrits avant qu'elle n'existe.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    /// L'empreinte forte, quand la source la publie — Modrinth — ou qu'elle a
    /// été calculée faute de mieux. Absente des verrous écrits avant qu'elle
    /// n'existe, d'où le `default` : ceux-là restent lisibles, et leur SHA-1
    /// continue de faire foi jusqu'au prochain `lock`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
    pub size: u64,
    pub side: String,
    /// En clair : demandé, dépendance déclarée, ou dépendance implicite.
    pub reason: String,
    /// `modId` fournis par ce jar, jars embarqués compris. Ce sont eux qui
    /// satisfont les dépendances des autres.
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMissing {
    pub mod_id: String,
    pub required_by: String,
    pub side: String,
}

impl LockedMod {
    /// La plus forte empreinte que le verrou porte pour ce jar.
    ///
    /// Le SHA-512 quand Modrinth le publie ou qu'on l'a calculé faute de
    /// mieux ; le SHA-1 pour ce que CurseForge est seul à donner, et pour les
    /// verrous écrits avant que le champ n'existe.
    pub fn checksum(&self) -> Option<mc_dl::Checksum> {
        self.sha512
            .clone()
            .map(mc_dl::Checksum::Sha512)
            .or_else(|| self.sha1.clone().map(mc_dl::Checksum::Sha1))
    }
}
