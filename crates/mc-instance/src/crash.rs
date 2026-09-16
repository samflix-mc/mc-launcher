//! Ce que Minecraft laisse derrière lui quand il s'arrête mal.
//!
//! Un joueur dont le jeu plante ne sait pas lire une trace Java et ne pensera
//! pas à joindre un fichier. Le launcher, lui, sait exactement où regarder :
//!
//! - `crash-reports/crash-*.txt` — écrit par le jeu quand il attrape
//!   l'exception. Le plus riche : description, trace, mods chargés, pilote
//!   graphique ;
//! - `logs/latest.log` — le reste du temps. Une erreur de chargement de mods
//!   ou un conflit de modules s'y trouve, alors qu'aucun rapport n'est produit
//!   car la JVM s'arrête avant que le jeu n'existe.
//!
//! Le second cas est le plus fréquent avec un modpack, et c'est justement
//! celui qu'aucun rapport de crash ne couvre.

mod lecture;
mod rapport;
mod surveillance;

pub use lecture::{parse, Crash};
pub use rapport::{find, now};
pub use surveillance::{loaded_mods, Watcher};
