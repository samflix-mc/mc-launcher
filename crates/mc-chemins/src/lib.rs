//! Où le launcher range ses affaires — un seul endroit qui le décide.
//!
//! ## Pourquoi ce crate existe
//!
//! Avant lui, quatre endroits dérivaient un chemin, chacun à sa façon :
//! `mc-dl` pour les données, `mc-auth` pour la session, `mc-log` pour les
//! journaux, `mc-instance` pour la racine des instances. Trois lisaient
//! l'environnement, chacun avec ses propres règles de repli, et rien ne
//! garantissait qu'ils tombent d'accord.
//!
//! Surtout, l'application graphique n'avait aucun moyen de leur imposer les
//! siens. Tauri expose un résolveur qui connaît les conventions de chaque
//! système — et qui n'est pas celui-là : il passe par la crate `dirs`, là où
//! les crates du dépôt lisent `std::env::var_os`. Trois cas connus les font
//! diverger, et le symptôme serait huit cents mégaoctets téléchargés une
//! seconde fois dans un répertoire voisin.
//!
//! ## Ce que le crate distingue
//!
//! Deux choses que la version d'avant confondait :
//!
//! - **Choisir les racines** — [`du_systeme`] — lit l'environnement, a une
//!   branche par plateforme, et PEUT diverger de ce que Tauri trouve.
//! - **Déduire l'arborescence** — [`depuis_bases`] — n'est que des `join`, et
//!   ne peut PAS diverger. C'est elle que l'application emploie sur les
//!   racines de Tauri.
//!
//! La comparaison des deux, journalisée en `warn` quand elle ne coïncide pas,
//! est ce qui permettra de savoir qu'on a un problème avant qu'un joueur ne le
//! signale.
//!
//! ## Une feuille, et ça compte
//!
//! Aucune dépendance hors `std`. C'est ce qui permet à `mc-dl` — dont tous les
//! autres dépendent — de dépendre de celui-ci sans créer de cycle, et ce qui
//! rend sa couverture entièrement atteignable par des tests purs.

mod emplacements;
mod pose;

pub use emplacements::{
    Bases, Emplacements, SEGMENT, bases_linux, bases_macos, bases_windows, depuis_bases, du_systeme,
};
pub use pose::{DejaPose, courants, poser, poses};
