//! Source CurseForge — consultée quand Modrinth ne connaît pas le projet.
//!
//! Elle vient en second pour une raison pratique : son API exige une clé,
//! nominative et à demander sur console.curseforge.com. Le launcher ne peut pas
//! en embarquer une — elle serait extraite du binaire et révoquée — donc
//! l'installation ne doit dépendre de CurseForge que lorsque c'est inévitable,
//! c'est-à-dire pour les mods qui n'y sont pas publiés ailleurs.
//!
//! Deux limites de la source, assumées ici plutôt que contournées :
//!
//! - elle ne dit pas si un mod est client, serveur ou les deux. Le côté est
//!   donc `both` par défaut, affiné ensuite par ce que déclare le jar ;
//! - un auteur peut interdire le téléchargement par un tiers
//!   (`downloadUrl: null`). Reconstruire l'URL du CDN contournerait ce refus :
//!   le cas est signalé avec le lien de la page, et le fichier attendu dans le
//!   dossier des apports manuels.

mod api;
mod candidats;
mod cle;
mod conversion;
mod recherche;
mod requetes;

use std::sync::Arc;

pub use cle::{api_key, config_key_path, is_key_error};

pub(crate) const API: &str = "https://api.curseforge.com/v1";

const GAME_MINECRAFT: u32 = 432;
/// Classe « Mods », par opposition aux modpacks et aux ressource packs.
const CLASS_MODS: u32 = 6;
/// `modLoaderType` de NeoForge.
const LOADER_NEOFORGE: u32 = 6;
const LOADER_FORGE: u32 = 1;
const LOADER_FABRIC: u32 = 4;
const LOADER_QUILT: u32 = 5;

/// `relationType` d'une dépendance obligatoire.
const RELATION_REQUIRED: u32 = 3;

/// Le client CurseForge, avec la clé d'API qu'il présente à chaque requête.
pub struct CurseForge {
    pub(crate) dl: Arc<mc_dl::Downloader>,
    pub(crate) key: String,
    /// Racine de l'API, substituable pour les tests : provoquer un refus de
    /// clé chez CurseForge n'est pas quelque chose qu'on peut demander.
    pub(crate) base: String,
}

impl CurseForge {
    pub fn new(dl: Arc<mc_dl::Downloader>, key: String) -> Self {
        Self::avec_base(dl, key, API)
    }

    pub(crate) fn avec_base(dl: Arc<mc_dl::Downloader>, key: String, base: &str) -> Self {
        Self {
            dl,
            key,
            base: base.to_string(),
        }
    }

    pub(crate) fn url(&self, chemin: &str) -> String {
        format!("{}{chemin}", self.base)
    }

    /// Construit la source si une clé est disponible, sinon `None`.
    pub fn from_env(dl: Arc<mc_dl::Downloader>) -> Option<Self> {
        api_key().map(|key| Self::new(dl, key))
    }
}
