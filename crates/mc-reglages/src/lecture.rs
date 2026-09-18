//! Lire et écrire le fichier de réglages.

use std::path::{Path, PathBuf};

use crate::types::Reglages;

/// Où les réglages vivent.
pub fn chemin() -> PathBuf {
    mc_chemins::courants().config.join("reglages.json")
}

/// Relit les réglages, ou rend les défauts.
///
/// **Ne rend jamais d'erreur.** Un fichier absent, illisible ou d'un schéma
/// inconnu donnent tous les défauts — perdre ses réglages est ennuyeux, ne pas
/// pouvoir ouvrir le launcher l'est davantage. Les cas sont distingués dans le
/// journal, pas dans le type de retour.
///
/// Ce qui est relu est TOUJOURS validé : le fichier s'édite à la main, et
/// c'est précisément ce qu'un joueur qui cherche des images par seconde fera.
pub fn charger(chemin: &Path) -> Reglages {
    let Ok(brut) = std::fs::read(chemin) else {
        tracing::debug!(fichier = %chemin.display(), "aucun réglage enregistré, défauts");
        return Reglages::default();
    };

    let mut reglages: Reglages = match serde_json::from_slice(&brut) {
        Ok(reglages) => reglages,
        Err(erreur) => {
            tracing::warn!(
                erreur = %erreur,
                fichier = %chemin.display(),
                "réglages illisibles : retour aux défauts"
            );
            return Reglages::default();
        }
    };

    if reglages.schema != crate::types::SCHEMA {
        // Pas un refus : `#[serde(default)]` sur chaque section fait que les
        // champs d'un schéma voisin se relisent, et ceux qu'on ne connaît pas
        // retombent sur leur défaut. C'est mieux que de tout perdre.
        tracing::info!(
            trouve = reglages.schema,
            attendu = crate::types::SCHEMA,
            "réglages d'un autre schéma : ce qui se relit est gardé"
        );
    }

    reglages.valider();
    reglages
}

/// Écrit les réglages, après les avoir validés.
///
/// La validation est ici et non chez l'appelant : elle ne doit pas pouvoir
/// être oubliée, et c'est le seul endroit par lequel un réglage entre sur le
/// disque.
pub fn enregistrer(chemin: &Path, reglages: &Reglages) -> anyhow::Result<Reglages> {
    let mut valides = reglages.clone();
    valides.valider();

    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(&valides)?;
    json.push('\n');
    mc_dl::write_atomic(chemin, json.as_bytes())?;

    // On rend ce qui a été ÉCRIT, et non ce qu'on a reçu : si une valeur a été
    // ramenée dans ses bornes, la fenêtre doit le montrer tout de suite.
    // Rendre l'entrée laisserait un curseur à une position que le fichier ne
    // porte pas, jusqu'au prochain rechargement.
    Ok(valides)
}

#[cfg(test)]
#[path = "lecture.test.rs"]
mod tests;
