//! Détection d'un runtime Java utilisable, sinon installation d'un Temurin.
//!
//! Minecraft 1.21.1 exige Java 21 : en dessous, le jeu s'arrête sur
//! `UnsupportedClassVersionError` avant même d'afficher une fenêtre. Un joueur
//! n'a aucune raison d'avoir un JDK, et celui qu'il a est souvent un 8 ou un 17
//! laissé par un vieux modpack.
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
pub use version::{parse_major, probe, Java, Origin, Version};
