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
    /// `modId` déclarés par le descripteur de ce jar. **Identité du mod.**
    ///
    /// C'est le seul ensemble qui fasse échouer NeoForge en double, et donc le
    /// seul sur lequel deux projets puissent être dits redondants.
    pub provides: BTreeSet<String>,
    /// `modId` apportés par les jars embarqués (JarJar).
    ///
    /// Ils satisfont des dépendances mais ne définissent aucune identité : deux
    /// mods embarquent légitimement la même bibliothèque, et NeoForge sait les
    /// dédupliquer au chargement. Les confondre avec [`provides`] faisait
    /// passer Sodium et Iris pour un doublon — ils partagent quatre shims
    /// Fabric — et supprimait l'un des deux en silence.
    ///
    /// [`provides`]: JarInfo::provides
    pub bundled: BTreeSet<String>,
    /// Dépendances obligatoires, hors plateforme.
    pub requires: Vec<Requirement>,
}

impl JarInfo {
    /// Tout ce que ce jar apporte, racine et embarqués confondus.
    ///
    /// C'est ce qui satisfait une dépendance — par opposition à [`provides`],
    /// qui dit qui est ce mod.
    ///
    /// [`provides`]: JarInfo::provides
    pub fn fournit(&self) -> impl Iterator<Item = &String> {
        self.provides.iter().chain(self.bundled.iter())
    }
}

mod analyse;

pub use analyse::parse_descriptor;
