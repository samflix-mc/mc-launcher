//! `META-INF/neoforge.mods.toml` : ce que le jar déclare de lui-même.
//!
//! C'est la seule source que le jeu lise réellement au démarrage — celle dont
//! l'écart avec les métadonnées d'API fait planter un client sur « Missing or
//! unsupported mods ».

use std::collections::BTreeSet;

use super::cote::Side;

/// Identifiants que NeoForge considère comme toujours présents : ils décrivent
/// la plateforme, pas un mod à installer.
pub const PLATFORM_IDS: &[&str] = &["minecraft", "neoforge", "forge", "java", "fml", "mcp"];

pub fn is_platform(mod_id: &str) -> bool {
    PLATFORM_IDS.contains(&mod_id.to_ascii_lowercase().as_str())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `modId` exigé, tel qu'écrit dans le descripteur.
    pub mod_id: String,
    /// Intervalle Maven, p. ex. `[21.1.65,)`. Conservé pour le diagnostic.
    pub version_range: Option<String>,
    pub side: Side,
}

/// Contenu utile d'un jar de mod.
#[derive(Debug, Clone, Default)]
pub struct JarInfo {
    /// `modId` que ce jar fournit, y compris ceux de ses jars embarqués.
    pub provides: BTreeSet<String>,
    /// Dépendances obligatoires, hors plateforme.
    pub requires: Vec<Requirement>,
}

mod analyse;

pub use analyse::parse_descriptor;
