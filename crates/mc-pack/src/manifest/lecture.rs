//! Lire un manifeste, l'écrire, et ce qu'on en tire.

use std::path::Path;

use anyhow::{Context, Result};
use mc_mods::Request;

use super::{Manifest, ModEntry};

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest> {
        let raw = std::fs::read(path)
            .with_context(|| format!("lecture du manifeste {}", path.display()))?;
        Manifest::parse(&raw).with_context(|| format!("manifeste {} illisible", path.display()))
    }

    /// Lit un manifeste qui n'a pas de chemin — celui d'une réponse HTTP.
    ///
    /// La vérification est la même que pour un fichier : ce qui arrive du
    /// réseau mérite moins de confiance, pas plus.
    pub fn parse(raw: &[u8]) -> Result<Manifest> {
        let manifest: Manifest = serde_json::from_slice(raw)?;
        manifest.check()?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        mc_dl::write_atomic(path, json.as_bytes())
    }

    /// Refuse un manifeste incohérent avant toute écriture sur le disque.
    ///
    /// Un manifeste fautif découvert après 800 Mo de téléchargement coûte
    /// beaucoup plus qu'une vérification à la lecture.
    pub fn requests(&self) -> Result<Vec<Request>> {
        self.mods.iter().map(ModEntry::to_request).collect()
    }

    /// Version majeure de Java à garantir.
    ///
    /// Trois sources, dans cet ordre, et la priorité est la règle : ce que le
    /// MANIFESTE déclare l'emporte sur ce que Mojang exige, parce qu'un pack
    /// peut avoir de bonnes raisons d'imposer une autre majeure que celle du
    /// jeu nu — un mod qui ne compile qu'avec, un bogue de JVM à contourner.
    ///
    /// `Option` en entrée : les descripteurs d'avant la 1.17 n'ont pas de bloc
    /// `javaVersion`. Le 21 final est le seul endroit du dépôt où ce chiffre
    /// est écrit en dur, et il l'est ici plutôt que dans trois appelants.
    pub fn java_major(&self, mojang_exige: Option<u32>) -> u32 {
        self.java.or(mojang_exige).unwrap_or(21)
    }
}

#[cfg(test)]
#[path = "lecture.test.rs"]
mod tests;

#[cfg(test)]
#[path = "lecture.demandes.test.rs"]
mod tests_demandes;
