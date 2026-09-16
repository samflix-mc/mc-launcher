//! Où vit le launcher, et où vit chaque instance.

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


#[cfg(test)]
#[path = "disposition.test.rs"]
mod tests;
