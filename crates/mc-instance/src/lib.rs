//! Installation d'une instance Minecraft : fichiers Mojang, puis NeoForge.
//!
//! Le disque est organisé comme celui des launchers multi-instances, pour une
//! raison de place : les bibliothèques et les assets d'une version pèsent près
//! d'un gigaoctet et ne dépendent que de la version du jeu. Ils sont partagés,
//! et seules les données propres à une partie — mods, configuration,
//! sauvegardes — appartiennent à l'instance.
//!
//! ```text
//! <données>/
//!   shared/                  ← joue le rôle d'un .minecraft
//!     versions/1.21.1/        descripteur et client vanilla
//!     versions/neoforge-21.1.250/
//!     libraries/  assets/
//!   instances/<nom>/
//!     minecraft/              répertoire de jeu : mods, config, saves
//!   runtime/temurin-21/
//! ```
//!
//! Le répertoire `shared` a la forme d'un `.minecraft` parce que
//! l'installateur NeoForge l'exige : il y cherche le client vanilla à patcher
//! et y dépose ce qu'il fabrique.

pub mod launch;
pub mod neoforge;
pub mod vanilla;

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Emplacements du launcher sur le disque.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            root: mc_dl::data_dir(),
        }
    }
}

impl Layout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Répertoire de forme `.minecraft`, partagé par toutes les instances.
    pub fn shared(&self) -> PathBuf {
        self.root.join("shared")
    }

    pub fn runtime(&self) -> PathBuf {
        self.root.join("runtime")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn instance(&self, name: &str) -> Instance {
        let dir = self.root.join("instances").join(name);
        Instance {
            name: name.to_string(),
            game_dir: dir.join("minecraft"),
            dir,
        }
    }
}

/// Une instance : tout ce qui lui est propre.
#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub dir: PathBuf,
    /// Répertoire de jeu passé à Minecraft ; contient `mods`, `config`,
    /// `saves`, `options.txt`.
    pub game_dir: PathBuf,
}

impl Instance {
    pub fn mods_dir(&self) -> PathBuf {
        self.game_dir.join("mods")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.game_dir.join("config")
    }

    pub fn create(&self) -> Result<()> {
        for dir in [&self.game_dir, &self.mods_dir(), &self.config_dir()] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("création de {}", dir.display()))?;
        }
        Ok(())
    }
}

/// Vérifie qu'une installation est complète et intacte.
///
/// `deep` recontrôle l'empreinte de chaque objet d'assets, ce que
/// l'installation ne fait pas pour ne pas relire 800 Mo à chaque lancement.
pub fn verify(
    mc: &str,
    neoforge_version: &str,
    layout: &Layout,
    deep: bool,
) -> Result<Vec<String>> {
    let shared = layout.shared();
    let mut problems = Vec::new();

    let version_json = shared.join("versions").join(mc).join(format!("{mc}.json"));
    let client_jar = shared.join("versions").join(mc).join(format!("{mc}.jar"));
    for path in [&version_json, &client_jar] {
        if !path.is_file() {
            problems.push(format!("fichier manquant : {}", path.display()));
        }
    }

    let neoforge_json = shared
        .join("versions")
        .join(neoforge::version_id(neoforge_version))
        .join(format!("{}.json", neoforge::version_id(neoforge_version)));
    if !neoforge_json.is_file() {
        problems.push(format!(
            "NeoForge {neoforge_version} n'est pas installé : {} absent",
            neoforge_json.display()
        ));
    }

    // Les deux descripteurs sont contrôlés : celui de NeoForge ajoute une
    // cinquantaine de bibliothèques au classpath, et il en manque une suffit à
    // faire échouer le démarrage aussi sûrement qu'une bibliothèque vanilla.
    for descriptor in [&version_json, &neoforge_json] {
        if !descriptor.is_file() {
            continue;
        }
        for library in vanilla::classpath(descriptor, &shared)? {
            if !library.is_file() {
                problems.push(format!("bibliothèque manquante : {}", library.display()));
            }
        }
    }

    if deep && problems.is_empty() {
        let index = shared.join("assets").join("indexes");
        let id = std::fs::read_dir(&index)
            .ok()
            .and_then(|entries| {
                entries.flatten().find_map(|e| {
                    e.path()
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                })
            })
            .unwrap_or_default();
        if !id.is_empty() {
            let report = vanilla::verify_assets(&shared, &id)?;
            for hash in report.missing {
                problems.push(format!("asset manquant : {hash}"));
            }
            for hash in report.corrupt {
                problems.push(format!("asset corrompu : {hash}"));
            }
        }
    }

    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_instances_ne_partagent_que_le_commun() {
        let layout = Layout::new(PathBuf::from("/data"));
        let une = layout.instance("samflix");
        let autre = layout.instance("essai");

        assert_eq!(
            une.game_dir,
            PathBuf::from("/data/instances/samflix/minecraft")
        );
        assert_eq!(
            une.mods_dir(),
            PathBuf::from("/data/instances/samflix/minecraft/mods")
        );
        assert_ne!(une.mods_dir(), autre.mods_dir());
        // Bibliothèques et assets, eux, sont communs.
        assert_eq!(layout.shared(), PathBuf::from("/data/shared"));
    }
}
