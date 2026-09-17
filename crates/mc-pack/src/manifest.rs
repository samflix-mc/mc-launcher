//! Le manifeste : ce qu'un pack est, écrit une fois et versionné.
//!
//! Un seul fichier décrit l'installation entière — version du jeu, chargeur,
//! version de Java, liste des mods. Il est volontairement court : tout ce qui
//! peut être déduit l'est, et ce qui est écrit à la main est ce qu'un humain a
//! décidé.
//!
//! Chaque mod peut être laissé libre (« la dernière version compatible ») ou
//! **épinglé** sur un build précis. Les deux ont leur place : le pack de
//! développement suit les mises à jour, celui de production ne bouge que
//! lorsqu'on le décide. C'est le rôle de `file`, qui désigne un build exact.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod controle;
mod entrees;
#[cfg(test)]
mod essais;
mod lecture;
mod serveur;
mod serveurs;

pub use entrees::{Loader, ModEntry};
pub use serveur::Server;

/// Version du format. Un manifeste d'une autre version se refuse plutôt que de
/// se lire à moitié.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Version de Minecraft, p. ex. `1.21.1`.
    pub minecraft: String,
    pub loader: Loader,
    /// Version majeure de Java. À défaut, celle qu'exige Mojang pour cette
    /// version du jeu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<u32>,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
    /// Où se connecter, par environnement.
    ///
    /// Le manifeste est le même partout : c'est la même image de contenu,
    /// servie sous trois noms. Ce n'est donc pas lui qui peut choisir, c'est le
    /// client — avec son propre environnement, celui que la CI lui a figé à la
    /// compilation. Un launcher de dev rejoint le serveur de dev.
    ///
    /// Les clés sont celles de `mc_log::Environment` : `development`,
    /// `preproduction`, `production`. Une absence n'est pas une erreur — la
    /// préproduction n'a pas de serveurs Minecraft derrière elle, et le jeu s'y
    /// lance sans rejoindre quoi que ce soit.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub servers: BTreeMap<String, Server>,
}
