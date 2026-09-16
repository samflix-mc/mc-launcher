//! Installation de NeoForge, par son installateur officiel.
//!
//! Le descripteur de version de NeoForge ne suffit pas à installer le
//! chargeur : une partie des bibliothèques n'existe pas telle quelle sur un
//! dépôt Maven, elle est *fabriquée* au moment de l'installation par une suite
//! de traitements — application de patchs binaires au client vanilla,
//! découpage du jar, renommage des symboles. Ces traitements sont des jars
//! livrés avec l'installateur, et leur enchaînement change d'une version à
//! l'autre.
//!
//! Les réimplémenter reviendrait à suivre indéfiniment un format interne. On
//! exécute donc l'installateur publié, avec le Java que le launcher vient de
//! garantir. Il est idempotent, ce qui permet de le relancer sans risque.

mod installateur;
mod versions;

pub use installateur::{install_client, install_server, version_id};
pub use versions::{latest_for, series_for};

pub(crate) const MAVEN: &str = "https://maven.neoforged.net/releases";
