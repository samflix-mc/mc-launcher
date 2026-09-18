//! Comment l'application impose ses emplacements aux crates.
//!
//! ## Le problème que ça résout
//!
//! Chaque crate dérivait son chemin de son côté, en relisant l'environnement.
//! L'application, elle, a un résolveur — celui de Tauri — qui n'emploie pas
//! les mêmes règles : il passe par la crate `dirs`, là où les crates du dépôt
//! lisent `std::env::var_os`. Trois cas connus les font diverger : un
//! `XDG_DATA_HOME` relatif, un `HOME` absent, un profil Windows itinérant.
//!
//! Deux emplacements différents pour les mêmes données, c'est huit cents
//! mégaoctets téléchargés deux fois et une session qu'on ne retrouve pas.
//!
//! ## Pourquoi une pose, et pas un argument
//!
//! Faire descendre les emplacements de main en main jusqu'à `mc-auth` aurait
//! demandé de changer la signature d'une trentaine de fonctions, dont
//! plusieurs sont l'interface publique d'un crate. Une pose unique, faite au
//! démarrage avant tout le reste, coûte un `OnceLock` et ne change aucune
//! signature.
//!
//! Le prix est nommé : c'est un état global, et il est donc posé UNE fois,
//! jamais remplacé, et jamais lu avant la pose dans le chemin normal.

use std::sync::OnceLock;

use crate::Emplacements;

/// La pose a déjà eu lieu.
///
/// Un type et non un `bool` : le seul appelant légitime est le démarrage de
/// l'application, et une seconde pose est un bogue de séquencement qu'il faut
/// voir, pas une situation à rattraper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DejaPose;

impl std::fmt::Display for DejaPose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "les emplacements ont déjà été posés ; ils ne se remplacent pas en cours d'exécution"
        )
    }
}

impl std::error::Error for DejaPose {}

static POSES: OnceLock<Emplacements> = OnceLock::new();

/// Impose les emplacements, une fois pour toute la durée du processus.
///
/// À appeler AVANT le premier `courants()`, c'est-à-dire avant l'ouverture du
/// journal et avant toute commande. Une pose tardive ne remplacerait rien et
/// laisserait deux moitiés du programme sur deux arborescences.
pub fn poser(emplacements: Emplacements) -> Result<(), DejaPose> {
    POSES.set(emplacements).map_err(|_| DejaPose)
}

/// Les emplacements en vigueur.
///
/// ## Le repli, et pourquoi il ne mémorise pas
///
/// Sans pose, on recalcule [`crate::du_systeme`] À CHAQUE APPEL. Ce n'est pas
/// un oubli d'optimisation : c'est le comportement d'aujourd'hui, celui que
/// sept suites de tests traversent en cours de processus — dont
/// `mc-pack/src/commandes/arguments.test.rs`, par `Options::default()`. Un
/// repli MÉMORISÉ figerait l'environnement du tout premier appelant, et un
/// test qui déplace `XDG_DATA_HOME` verrait selon son rang dans la suite.
///
/// Le coût est de quelques `join` : ce chemin n'est pas chaud.
pub fn courants() -> Emplacements {
    match POSES.get() {
        Some(poses) => poses.clone(),
        None => crate::du_systeme(),
    }
}

/// Les emplacements ont-ils été imposés ?
///
/// Pour le diagnostic seulement : « ce que Tauri a donné » ou « ce que
/// l'environnement dit » n'est pas la même réponse à la même question, et
/// c'est la première chose à savoir quand un joueur ne retrouve pas ses
/// données.
pub fn poses() -> bool {
    POSES.get().is_some()
}

#[cfg(test)]
#[path = "pose.test.rs"]
mod tests;
