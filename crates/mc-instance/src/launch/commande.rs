//! La ligne de commande produite, et comment on l'affiche sans tout révéler.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Command {
    pub java: PathBuf,
    pub args: Vec<String>,
    /// Répertoire de travail : le jeu y écrit `saves`, `logs`, `options.txt`.
    pub working_dir: PathBuf,
}

impl Command {
    /// Ligne de commande lisible, pour l'afficher ou la rejouer à la main.
    ///
    /// Le classpath est abrégé : il fait plusieurs dizaines de milliers de
    /// caractères et personne ne le lit.
    pub fn display(&self) -> String {
        let mut out = vec![self.java.display().to_string()];
        let mut skip_next = false;
        for arg in &self.args {
            if skip_next {
                out.push(format!("<{} bibliothèques>", arg.split(':').count()));
                skip_next = false;
                continue;
            }
            if arg == "-cp" {
                skip_next = true;
            }
            out.push(arg.clone());
        }
        out.join(" ")
    }
}

// --- Lecture des descripteurs ------------------------------------------------

#[cfg(test)]
#[path = "commande.test.rs"]
mod tests;
