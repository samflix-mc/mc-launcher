//! Where the launcher lives, and where each instance lives.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Locations of the launcher on disk.
#[derive(Debug, Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            root: mc_paths::current().data,
        }
    }
}

impl Layout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Directory shaped like `.minecraft`, shared by all instances.
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

/// An instance: everything that belongs to it alone.
#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub dir: PathBuf,
    /// Game directory passed to Minecraft; contains `mods`, `config`,
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
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "layout.test.rs"]
mod tests;
