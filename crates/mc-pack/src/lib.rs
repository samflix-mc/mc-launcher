//! Enchaînement complet : d'un manifeste JSON à une instance jouable.
//!
//! L'ordre des étapes n'est pas arbitraire, chacune dépend de la précédente :
//!
//! 1. **le chargeur** — `latest` est résolu tout de suite, pour que le verrou
//!    consigne une version exacte et non un mot ;
//! 2. **les fichiers de Mojang** — ils donnent au passage la version de Java
//!    qu'exige cette version du jeu ;
//! 3. **Java** — détecté ou installé, en s'appuyant sur ce que Mojang exige ;
//! 4. **NeoForge** — son installateur patche le client vanilla et a besoin du
//!    Java de l'étape précédente ;
//! 5. **les mods** — résolus, téléchargés, puis répartis entre client et
//!    serveur ;
//! 6. **le verrou** — écrit en dernier, il décrit ce qui a réellement été fait.

mod coherence;
mod installation;
mod verification;

pub mod jeu;
pub mod lockfile;
pub mod manifest;
pub mod progression;
pub mod source;

pub use coherence::mods_client_absents;
pub use installation::install;
pub use jeu::{Identite, Partie, jouer, preparer};
pub use progression::{Etape, Muet, Rapport};
pub use verification::verify;

use lockfile::Lockfile;
use std::path::PathBuf;

/// Ce qu'une installation a produit, pour le compte rendu.
#[derive(Debug)]
pub struct Outcome {
    pub instance: mc_instance::Instance,
    pub server_dir: PathBuf,
    pub java: mc_java::Java,
    pub neoforge: String,
    pub assets_downloaded: usize,
    pub libraries: usize,
    pub client_mods: usize,
    pub server_mods: usize,
    pub removed: Vec<String>,
    pub lock: Lockfile,
    pub lock_path: PathBuf,
    pub previous_lock: Option<Lockfile>,
    /// D'où venait le pack, tel qu'on l'a demandé.
    pub source: String,
    /// Le pack distant était injoignable et la copie locale a servi.
    pub from_cache: bool,
}

#[derive(Debug, Default)]
pub struct Options {
    /// Rejouer exactement le verrou au lieu de rechercher les versions.
    pub locked: bool,
    /// Installer aussi un serveur NeoForge complet, et pas seulement ses mods.
    pub with_server: bool,
    /// Nom de l'instance ; par défaut, celui du pack.
    pub instance_name: Option<String>,
    pub layout: mc_instance::Layout,
}

#[cfg(test)]
mod essais;
