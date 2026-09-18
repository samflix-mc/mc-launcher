//! Ce que le joueur règle, et où cela vit.
//!
//! ## Qui lit ce fichier
//!
//! `mc-app`, et lui seul. `mc-pack` ne dépend PAS de ce crate : la
//! bibliothèque d'installation ne doit pas savoir qu'une interface existe, et
//! le CLI garde ses propres drapeaux. L'application lit les réglages et passe
//! les VALEURS à `preparer` — pas la structure.
//!
//! C'est ce qui permet de lancer une partie depuis la ligne de commande sans
//! qu'un fichier de réglages écrit par la fenêtre ne s'en mêle.
//!
//! ## Où
//!
//! Dans la CONFIGURATION et non dans les données : ce sont des préférences,
//! petites et précieuses, pas un cache reconstructible. Sous Linux, cela veut
//! dire qu'effacer les huit cents mégaoctets d'instances ne les emporte pas.
//! Ailleurs, si — voir `docs/authentification.md`.

mod bornes;
mod lecture;
mod options_txt;
mod types;

pub use bornes::{ECHELLE, FPS, HAUTEUR, LARGEUR, MEMOIRE, RENDU, SIMULATION, VOILE_PLANCHER};
pub use lecture::{charger, chemin, enregistrer};
pub use options_txt::fusionner;
pub use types::{Apparence, Fenetre, Fond, Jeu, Lanceur, ModeFenetre, Reglages, SCHEMA};
