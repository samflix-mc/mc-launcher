//! Ce que la résolution rend : les mods retenus, et ce qui manque.

use crate::Candidate;
use crate::jar::Side;
use std::collections::BTreeSet;
use std::path::PathBuf;

use super::raison::Reason;

/// Un mod résolu, téléchargé et analysé.
#[derive(Debug, Clone)]
pub struct Installed {
    pub candidate: Candidate,
    /// Côté effectif, après combinaison du manifeste, des métadonnées du
    /// projet et de ce que le descripteur du jar réclame.
    pub side: Side,
    pub reason: Reason,
    /// Chemin dans le cache du launcher.
    pub path: PathBuf,
    /// `modId` que ce jar fournit, jars embarqués compris.
    pub provides: BTreeSet<String>,
    /// `modId` que ce jar exige pour démarrer, hors plateforme.
    pub requires: Vec<crate::jar::Requirement>,
    /// Au nom de quoi ce build occupe la place — voir [`autorite`].
    ///
    /// Porté par l'entrée retenue plutôt que par une seconde table indexée de
    /// la même façon : deux tables à tenir en phase, c'est une occasion de les
    /// laisser diverger, et `reason` part dans le verrou.
    pub(super) autorite: u8,
}

/// Résultat complet d'une résolution.
#[derive(Debug, Default)]
pub struct Plan {
    pub mods: Vec<Installed>,
    /// Dépendances exigées par un jar qu'aucune source n'a su fournir.
    /// Non bloquant ici : l'appelant décide d'arrêter ou d'avertir.
    pub unresolved: Vec<Unresolved>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    pub mod_id: String,
    pub required_by: String,
    pub side: Side,
}

impl Plan {
    pub fn for_side(&self, side: Side) -> impl Iterator<Item = &Installed> {
        self.mods.iter().filter(move |m| m.side.includes(side))
    }
}

#[cfg(test)]
#[path = "plan.test.rs"]
mod tests;
