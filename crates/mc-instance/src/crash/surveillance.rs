//! Suivre le journal d'une partie en cours.

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::lecture::{Crash, EXCERPT_LINES, split_exception, strip_ansi};

#[derive(Debug, Default)]
pub struct Watcher {
    /// Exception en cours d'accumulation et nombre de lignes déjà prises.
    pending: Option<(Crash, usize)>,
    found: Vec<Crash>,
    /// Évite de remonter cent fois la même : un mod qui échoue à chaque tick
    /// remplirait le tableau de bord à lui seul.
    seen: std::collections::BTreeSet<String>,
}

/// Au-delà, on cesse de remonter : une session qui produit tant d'exceptions
/// distinctes a un problème global, que les premières décrivent déjà.
const MAX_DISTINCT: usize = 5;

impl Watcher {
    pub fn new() -> Watcher {
        Watcher::default()
    }

    /// Donne une ligne de la sortie du jeu.
    pub fn line(&mut self, line: &str) {
        let clean = strip_ansi(line);

        // Une trace se poursuit par ses cadres ; tant qu'ils arrivent, ils
        // appartiennent à l'exception en cours.
        if let Some((crash, taken)) = &mut self.pending {
            let trimmed = clean.trim_start();
            let continues = trimmed.starts_with("at ")
                || trimmed.starts_with("Caused by:")
                || trimmed.starts_with("... ")
                || trimmed.starts_with("Suppressed:");
            if continues && *taken < EXCERPT_LINES {
                crash.excerpt.push('\n');
                crash.excerpt.push_str(&clean);
                *taken += 1;
                return;
            }
            let (finished, _) = self.pending.take().expect("présent");
            self.keep(finished);
        }

        if self.found.len() >= MAX_DISTINCT {
            return;
        }
        if let Some((exception, message)) = split_exception(&clean) {
            self.pending = Some((
                Crash {
                    exception,
                    message,
                    excerpt: clean,
                    source: PathBuf::from("sortie du jeu"),
                },
                0,
            ));
        }
    }

    fn keep(&mut self, crash: Crash) {
        let key = format!("{}: {}", crash.exception, crash.message);
        if self.seen.insert(key) {
            self.found.push(crash);
        }
    }

    /// Exceptions distinctes relevées pendant l'exécution.
    pub fn finish(mut self) -> Vec<Crash> {
        if let Some((crash, _)) = self.pending.take() {
            self.keep(crash);
        }
        self.found
    }
}

/// Liste des mods chargés, pour accompagner un rapport.
///
/// Un plantage de modpack vient presque toujours d'un mod ou d'un couple de
/// mods ; savoir lesquels étaient présents épargne un aller-retour.
pub fn loaded_mods(game_dir: &Path) -> Result<Vec<String>> {
    let mods = game_dir.join("mods");
    let Ok(entries) = std::fs::read_dir(&mods) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".jar"))
        .collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
#[path = "surveillance.test.rs"]
mod tests;
