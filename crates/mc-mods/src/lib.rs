//! Résolution et téléchargement des mods d'un pack.
//!
//! Le manifeste nomme quelques mods ; le dossier `mods` en contient toujours
//! davantage. L'écart, ce sont les dépendances — et le travail de ce crate est
//! de le combler sans intervention.
//!
//! Trois sources d'information sont croisées, dans cet ordre de fiabilité
//! croissante :
//!
//! 1. **ce que le manifeste demande** — éventuellement un build épinglé ;
//! 2. **ce que l'API déclare** — les dépendances saisies par l'auteur au
//!    moment de la publication, souvent incomplètes ;
//! 3. **ce que le jar exige** — `META-INF/neoforge.mods.toml`, la seule source
//!    que le jeu lise réellement.
//!
//! Les mods sont cherchés dans trois sources, de la plus sûre à la moins
//! contractuelle : [`modrinth`], puis [`curseforge`] si une clé d'API est
//! configurée, puis [`curseforge_web`] — l'API du site, sans clé, avec les
//! limites que son module détaille.
//!
//! Le troisième point est celui qui décide : après téléchargement, chaque jar
//! est ouvert, ses `modId` obligatoires comparés à ceux que le pack fournit, et
//! tout manque relance un tour de résolution. On s'arrête quand plus rien ne
//! manque — ce qui est exactement la condition que NeoForge vérifiera au
//! démarrage.

pub mod curseforge;
pub mod curseforge_web;
pub mod jar;
pub mod modrinth;
pub mod resolve;

pub use jar::Side;
pub use resolve::{Installed, Options, Plan, Reason, Registry, Request, resolve, resolve_with};

use serde::{Deserialize, Serialize};

/// D'où vient un fichier. Détermine l'API à interroger pour ses dépendances :
/// un identifiant de projet Modrinth n'a aucun sens chez CurseForge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Modrinth,
    CurseForge,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Modrinth => "modrinth",
            Origin::CurseForge => "curseforge",
        }
    }
}

/// Canal de publication.
///
/// Un pack de production s'en tient aux `release`. Autoriser les `beta` se
/// décide mod par mod dans le manifeste : certains mods très suivis ne
/// publient qu'en beta pendant des mois, et les interdire bloquerait le pack
/// sur une version d'il y a un an.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Release,
    Beta,
    Alpha,
}

impl Channel {
    pub fn parse(text: &str) -> Channel {
        match text.trim().to_ascii_lowercase().as_str() {
            "release" => Channel::Release,
            "beta" => Channel::Beta,
            _ => Channel::Alpha,
        }
    }

    /// `self` est-il acceptable quand le manifeste autorise au plus `limit` ?
    pub fn allowed_by(self, limit: Channel) -> bool {
        self <= limit
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Release => "release",
            Channel::Beta => "beta",
            Channel::Alpha => "alpha",
        }
    }
}

/// Dépendance telle que l'API la déclare.
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
mod tests {
    use super::*;

    fn candidat(sha1: Option<&str>, sha512: Option<&str>) -> Candidate {
        Candidate {
            origin: Origin::Modrinth,
            project_id: "p".into(),
            slug: "s".into(),
            name: "N".into(),
            version_id: "v".into(),
            version_number: "1.0".into(),
            display_name: "1.0".into(),
            channel: Channel::Release,
            file_name: "s.jar".into(),
            url: "https://exemple.invalid/s.jar".into(),
            sha1: sha1.map(str::to_string),
            sha512: sha512.map(str::to_string),
            size: 0,
            published: "2025-01-01".into(),
            project_side: Side::Both,
            declared_deps: Vec::new(),
            page_url: None,
            redistributable: true,
        }
    }

    /// Ce qui vaut pour la majeure partie d'un pack : Modrinth donne les deux,
    /// et c'est la forte qu'on oppose au fichier téléchargé.
    #[test]
    fn l_empreinte_forte_prime_sur_le_sha1() {
        let c = candidat(Some("aa"), Some("bb"));
        assert_eq!(c.checksum(), Some(mc_dl::Checksum::Sha512("bb".into())));
    }

    /// CurseForge ne publie rien de plus fort : refuser le SHA-1 reviendrait à
    /// installer ses jars sans les vérifier du tout.
    #[test]
    fn a_defaut_le_sha1_reste_opposable() {
        let c = candidat(Some("aa"), None);
        assert_eq!(c.checksum(), Some(mc_dl::Checksum::Sha1("aa".into())));
        assert_eq!(candidat(None, None).checksum(), None);
    }

    #[test]
    fn un_canal_plus_stable_que_la_limite_est_accepte() {
        assert!(Channel::Release.allowed_by(Channel::Beta));
        assert!(Channel::Beta.allowed_by(Channel::Beta));
        assert!(!Channel::Alpha.allowed_by(Channel::Beta));
        assert!(!Channel::Beta.allowed_by(Channel::Release));
    }
}
