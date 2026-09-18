//! Du manifeste au dossier `mods` : résolution, téléchargement, rattrapage.
//!
//! Le cycle est volontairement itératif plutôt que récursif sur les seules
//! métadonnées :
//!
//! ```text
//!   demandes du manifeste
//!        ↓  résolution (Modrinth, puis CurseForge)
//!   candidats + dépendances déclarées
//!        ↓  téléchargement vérifié
//!   jars sur le disque
//!        ↓  lecture des neoforge.mods.toml
//!   modId fournis / modId exigés
//!        ↓  écart non vide ? → nouveau tour
//!   plan stable
//! ```
//!
//! Le dernier tour est celui qui compte : il attrape les dépendances qu'aucune
//! API ne déclare. C'est le cas courant — un auteur qui ajoute une bibliothèque
//! entre deux versions ne revient pas éditer la fiche de publication — et c'est
//! exactement ce qui fait planter un client au démarrage avec un écran
//! « Missing or unsupported mods ».

mod boucle;
mod choix;
mod demande;
mod deploiement;
mod doublons;
#[cfg(test)]
mod essais;
mod file;
mod inspection;
mod options;
mod plan;
mod raison;
mod rattrapage;
mod registre;
mod telechargement;

pub use boucle::{resolve, resolve_with};
pub use demande::Request;
pub use deploiement::{Deployed, deploy};
pub use options::Options;
pub use plan::{Installed, Plan, Unresolved};
pub use raison::Reason;
pub use registre::{Progres, Registry};

/// Nombre de tours de rattrapage.
///
/// Une chaîne de dépendances implicites dépasse rarement deux niveaux ; la
/// borne protège d'une boucle si deux mods se réclament mutuellement sans que
/// la recherche converge.
const MAX_PASSES: usize = 6;

/// Téléchargements simultanés. Modrinth limite le débit par agent : au-delà
/// d'une poignée de connexions, les 429 coûtent plus de temps qu'ils n'en font
/// gagner.
const PARALLEL_DOWNLOADS: usize = 6;
