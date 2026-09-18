//! Le geste unique : vérifier, rattraper s'il le faut, puis jouer.
//!
//! ## Ce que cela renverse, et pourquoi
//!
//! `docs/lancement.md`, `docs/interface.md` et l'en-tête de `jeu.rs` posaient
//! « installer et jouer restent deux gestes », avec un motif juste : enchaîner
//! les deux ferait attendre huit cents mégaoctets à qui voulait seulement
//! jouer.
//!
//! Ce module RÉSOUT ce motif au lieu de le contredire. Depuis qu'une
//! comparaison d'empreintes coûte quelques dizaines de kilooctets, le cas
//! courant — rien n'a bougé — ne fait plus attendre personne. Et le cas où
//! quelque chose a bougé est précisément celui où ne rien faire donnerait une
//! éjection à la connexion, sans message utile.
//!
//! Le CLI garde ses deux commandes : `install` et `launch` sont des usages
//! d'outilleur, où l'on veut décider soi-même de ce qui se passe.
//!
//! ## Le nom
//!
//! Pas `rejoindre` : ce verbe est déjà pris par le serveur cible
//! (`jeu::cible::choisir`, `QuickPlay::Multiplayer`, `Partie.cible`), et deux
//! sens pour un mot dans le même crate se paient à chaque relecture.

use std::sync::Arc;

use anyhow::Result;

use crate::comparaison::{Ecart, EtatDuPack};
use crate::progression::Rapport;
use crate::source::Source;
use crate::{Options, Outcome};

/// Ce qui s'est réellement passé.
///
/// Rend un COMPTE RENDU et non une chaîne, et c'est une exigence du front :
/// trois champs qu'il affiche n'ont pas d'autre source. `ecarts` et
/// `hors_ligne` viennent de l'état du pack ; `introuvables` est un résultat de
/// résolution, lisible sur aucun disque — il n'existe que dans le verrou que
/// l'installation vient d'écrire.
///
/// Une chaîne obligerait la fenêtre à relire un message pour en tirer une
/// liste, ce qui est exactement ce qu'on ne veut pas d'un pont.
#[derive(Debug)]
pub struct Deroulement {
    /// Ce que la comparaison a vu avant d'agir.
    pub etat: EtatDuPack,
    /// L'installation, si elle a eu lieu.
    pub installation: Option<Outcome>,
    /// Le rapport de la partie, si le jeu a été lancé.
    pub partie: Option<mc_instance::launch::Report>,
}

impl Deroulement {
    /// Les dépendances qu'aucune source n'a su fournir.
    ///
    /// Vide quand rien n'a été installé — on ne prétend pas savoir ce qu'une
    /// résolution qui n'a pas eu lieu aurait trouvé.
    pub fn introuvables(&self) -> Vec<String> {
        self.installation
            .as_ref()
            .map(|pose| {
                pose.lock
                    .unresolved
                    .iter()
                    .map(|manque| manque.mod_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Ce par quoi l'installation s'est écartée de ce qui était attendu.
    pub fn ecarts(&self) -> Vec<String> {
        self.installation
            .as_ref()
            .map(|pose| pose.ecarts.clone())
            .unwrap_or_default()
    }
}

/// Vérifie, rattrape s'il le faut, puis lance la partie.
///
/// L'ordre n'est pas négociable : c'est la comparaison qui décide, et elle
/// passe avant tout. Lancer d'abord et vérifier ensuite ferait entrer le
/// joueur avec des registres NeoForge qui ne concordent plus.
#[allow(clippy::too_many_arguments)]
pub async fn mettre_a_jour_et_jouer(
    source: &Source,
    options: &Options,
    identite: super::Identite,
    serveur: Option<String>,
    memoire: Option<u32>,
    rapport: Arc<dyn Rapport>,
) -> Result<Deroulement> {
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;

    // C'est pendant la comparaison que la fenêtre a l'air figée : quelques
    // centaines de millisecondes de réseau sans qu'aucune étape ne s'allume.
    rapport.note("Vérification du pack…");
    let etat = crate::comparaison::comparer(source, options, &dl).await;

    let installation = if doit_rattraper(&etat) {
        tracing::info!(ecart = ?etat.ecart, "rattrapage avant la partie");
        Some(crate::install(source, options, Arc::clone(&rapport)).await?)
    } else {
        tracing::info!(ecart = ?etat.ecart, "rien à rattraper");
        None
    };

    let partie = super::preparer(source, options, identite, serveur, memoire).await?;
    let compte_rendu = super::jouer(&partie).await?;

    Ok(Deroulement {
        etat,
        installation,
        partie: Some(compte_rendu),
    })
}

/// Faut-il installer avant de jouer ?
///
/// Fonction PURE, extraite du corps pour qu'elle soit éprouvable : le corps,
/// lui, lance Minecraft et attend qu'il se termine.
///
/// Hors ligne, on ne rattrape RIEN, et c'est le point qui se devine mal. On
/// n'a pas de quoi décider : `Ecart::Inconnu` ne veut pas dire « à jour », il
/// veut dire « on ne sait pas ». Installer sur cette base repartirait du cache
/// pour reposer ce qui est déjà là — plusieurs minutes de vérification
/// d'empreintes, sans rien apprendre, au moment précis où le joueur n'a pas de
/// réseau et veut seulement jouer.
pub fn doit_rattraper(etat: &EtatDuPack) -> bool {
    if etat.hors_ligne {
        return false;
    }
    match etat.ecart {
        Ecart::Absent | Ecart::MiseAJour | Ecart::Reinstallation => true,
        Ecart::AJour | Ecart::Inconnu => false,
    }
}

#[cfg(test)]
#[path = "enchainement.test.rs"]
mod tests;
