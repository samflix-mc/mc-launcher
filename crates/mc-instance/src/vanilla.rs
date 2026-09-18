//! Les fichiers publiés par Mojang : client, bibliothèques, assets.
//!
//! Tout part de `version_manifest_v2.json`, qui renvoie vers le descripteur
//! d'une version, lequel décrit le reste. Chaque fichier vient avec son SHA-1,
//! ce qui permet de tout vérifier sans faire confiance au transport.
//!
//! Le partage est délibéré : bibliothèques et assets vivent dans un répertoire
//! commun à toutes les instances. Ils représentent près d'un gigaoctet, ne
//! dépendent que de la version du jeu, et les dupliquer par instance rendrait
//! inutilisable le fait d'en avoir plusieurs.

mod assets;
mod bibliotheques;
mod descripteur;
mod installation;
mod plateforme;
mod regles;
mod verification;

pub(crate) use descripteur::{Features, Library, Rule};
pub(crate) use regles::{allowed, allowed_with};

pub use descripteur::Artifact;
pub use installation::{Vanilla, install, java_exige};
pub use plateforme::{maven_path, mojang_arch, mojang_os};
pub use verification::{VerifyReport, classpath, verify_assets};

pub(crate) const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub(crate) const RESOURCES: &str = "https://resources.download.minecraft.net";

/// Téléchargements simultanés pour les assets.
///
/// Ce sont quelques milliers de fichiers de quelques kilooctets : la latence
/// domine, et la concurrence est ce qui fait la différence entre deux minutes
/// et une demi-heure.
const PARALLEL: usize = 16;
