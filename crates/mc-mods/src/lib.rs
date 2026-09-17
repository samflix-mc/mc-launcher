//! Résolution et téléchargement des mods d'un pack.
//!
//! Le manifeste nomme quelques mods ; le dossier `mods` en contient toujours
//! davantage. L'écart, ce sont les dépendances — et le travail de ce crate est
//! de le combler sans intervention.
//!
//! Trois sources d'information sont croisées, dans cet ordre de fiabilité
//! croissante :
//!
//! 1. **ce que le manifeste demande** — éventuellement un build épinglé ;
//! 2. **ce que l'API déclare** — les dépendances saisies par l'auteur au
//!    moment de la publication, souvent incomplètes ;
//! 3. **ce que le jar exige** — `META-INF/neoforge.mods.toml`, la seule source
//!    que le jeu lise réellement.
//!
//! Les mods sont cherchés dans deux sources, de la plus sûre à la moins
//! contractuelle : [`modrinth`], puis [`curseforge_web`] — l'API publique du
//! site de CurseForge, sans clé, avec les limites que son module détaille.
//!
//! La Core API de CurseForge, celle qui demande une clé d'inscription, a été
//! retirée : le launcher n'en dépend plus, et personne n'a de clé à poser pour
//! installer un pack.
//!
//! Le troisième point est celui qui décide : après téléchargement, chaque jar
//! est ouvert, ses `modId` obligatoires comparés à ceux que le pack fournit, et
//! tout manque relance un tour de résolution. On s'arrête quand plus rien ne
//! manque — ce qui est exactement la condition que NeoForge vérifiera au
//! démarrage.
mod canal;
mod candidat;
mod origine;

pub mod curseforge_web;
pub mod jar;
pub mod modrinth;
pub mod resolve;

pub use canal::Channel;
pub use candidat::{Candidate, DeclaredDep};
pub use jar::Side;
pub use origine::Origin;
pub use resolve::{Installed, Options, Plan, Reason, Registry, Request, resolve, resolve_with};

#[cfg(test)]
mod essais;
