//! Détection du runtime Java EXACT que le pack exige, sinon installation d'un
//! Temurin.
//!
//! Minecraft 1.21.1 exige Java 21 : en dessous, le jeu s'arrête sur
//! `UnsupportedClassVersionError` avant même d'afficher une fenêtre. Un joueur
//! n'a aucune raison d'avoir un JDK, et celui qu'il a est souvent un 8 ou un 17
//! laissé par un vieux modpack.
//!
//! ## « Exactement », et non « au moins »
//!
//! La majeure demandée est celle que le VERROU porte — donc celle avec
//! laquelle NeoForge a été installé. Un Java plus récent n'est pas « assez
//! bon » : il change le comportement des mixins et le format des registres, et
//! le serveur tranche par une éjection qui ne nomme pas sa cause.
//!
//! Le launcher vérifie donc l'égalité, et à chaque lancement. Un poste qui a
//! un Java 22 et un pack qui demande 21 recevra un Temurin 21 dédié, sans
//! qu'on touche au 22 du système.
//!
//! La règle vaut aux DEUX bouts — [`detect`] et l'installation — et ce n'est
//! pas une symétrie décorative : sans elle, l'installation accepterait ce que
//! la détection refuse, et `ensure` réinstallerait cent quatre-vingts
//! mégaoctets à chaque lancement sans jamais converger.
//!
//! La stratégie est donc : chercher, vérifier, et n'installer qu'en dernier
//! recours — installer systématiquement coûterait 50 Mo à chaque poste pour
//! rien, et ne jamais installer renverrait l'utilisateur vers une page de
//! téléchargement, ce qui est précisément ce qu'un launcher doit éviter.
//!
//! Le runtime installé est **dédié au launcher** : il vit dans son répertoire
//! de données, n'est pas ajouté au `PATH`, et ne touche pas au Java du système.
mod adoptium;
mod archive;
mod detection;
mod emplacements;
mod installation;
mod version;

pub use detection::{detect, ensure};
pub use emplacements::{default_runtime_dir, managed_home};
pub use installation::install;
pub use version::{Java, Origin, Version, parse_major, probe};

#[cfg(test)]
mod essais;
