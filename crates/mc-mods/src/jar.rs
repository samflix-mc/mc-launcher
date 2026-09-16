//! Ce qu'un jar déclare vraiment, lu dans le jar et non dans une API.
//!
//! Les dépendances annoncées par Modrinth ou CurseForge sont saisies à la main
//! par l'auteur au moment de la publication. Elles sont souvent incomplètes :
//! une bibliothèque ajoutée entre deux versions, une dépendance considérée
//! comme évidente, un envoi automatisé qui ne remplit pas le champ. Le jeu, lui,
//! ne lit que `META-INF/neoforge.mods.toml` — et s'arrête au démarrage dès
//! qu'une dépendance obligatoire y manque.
//!
//! On lit donc la même source que NeoForge. Deux détails décident de la
//! justesse du résultat :
//!
//! - **JarJar** : un mod peut embarquer ses bibliothèques dans
//!   `META-INF/jarjar/`. Elles fournissent leur `modId` sans exister comme
//!   fichier séparé. Les ignorer ferait conclure à une dépendance manquante et
//!   installerait un doublon — deux versions du même mod, ce que NeoForge
//!   refuse ;
//! - **le `side` d'une dépendance** : une dépendance déclarée `side = "CLIENT"`
//!   n'a rien à faire dans le dossier `mods` du serveur.

pub(crate) mod cote;
mod descripteur;
mod lecture;

pub use cote::Side;
pub use descripteur::{is_platform, parse_descriptor, JarInfo, Requirement, PLATFORM_IDS};
pub use lecture::{inspect, inspect_bytes};
