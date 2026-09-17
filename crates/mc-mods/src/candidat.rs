//! Une version publiée, telle que le résolveur la manipule.

use crate::{Channel, Origin, Side};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredDep {
    /// Identifiant de projet **dans la source du parent**.
    pub project_id: String,
    /// Version précise exigée, quand la source la donne. Modrinth le fait
    /// parfois ; l'honorer évite d'installer une version plus récente qu'une
    /// autre dépendance interdit.
    pub version_id: Option<String>,
}

/// Une version publiée, candidate à l'installation.
///
/// Type commun aux deux sources : le résolveur ne sait pas d'où vient ce qu'il
/// manipule, ce qui évite de dupliquer sa logique par backend.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub origin: Origin,
    pub project_id: String,
    pub slug: String,
    /// Nom lisible du projet, p. ex. « Just Enough Items ».
    pub name: String,
    /// Identifiant de la version — `version_id` chez Modrinth, `fileId` chez
    /// CurseForge. C'est lui qu'on épingle dans le lockfile.
    pub version_id: String,
    pub version_number: String,
    pub display_name: String,
    pub channel: Channel,
    pub file_name: String,
    pub url: String,
    pub sha1: Option<String>,
    /// Publié par Modrinth à côté du SHA-1, ou calculé par nos soins quand la
    /// source ne publie rien.
    pub sha512: Option<String>,
    pub size: u64,
    /// Date ISO 8601, utilisée pour départager deux versions compatibles.
    pub published: String,
    pub project_side: Side,
    pub declared_deps: Vec<DeclaredDep>,
    pub page_url: Option<String>,
    /// `false` quand l'auteur a désactivé le téléchargement par un tiers.
    pub redistributable: bool,
}

impl Candidate {
    /// La plus forte empreinte que l'on ait sur ce fichier.
    ///
    /// Modrinth publie un SHA-512 à côté du SHA-1 : le préférer retire le
    /// SHA-1 du chemin de vérification pour la majeure partie d'un pack.
    /// CurseForge ne donne que du SHA-1 ou du MD5, et c'est tout ce qu'il y a
    /// à opposer au fichier qu'il sert — d'où le repli.
    pub fn checksum(&self) -> Option<mc_dl::Checksum> {
        self.sha512
            .clone()
            .map(mc_dl::Checksum::Sha512)
            .or_else(|| self.sha1.clone().map(mc_dl::Checksum::Sha1))
    }
}

#[cfg(test)]
#[path = "candidat.test.rs"]
mod tests;
