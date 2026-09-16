//! Plomberie partagée : téléchargement vérifié et emplacements de données.
//!
//! Installer un modpack, c'est récupérer quelques milliers de fichiers depuis
//! cinq domaines différents. Trois propriétés suffisent à rendre l'opération
//! sûre et relançable :
//!
//! - **vérifié** — chaque fichier est comparé à l'empreinte publiée par sa
//!   source. Un CDN qui renvoie une page d'erreur en HTTP 200 est détecté ici,
//!   pas trois heures plus tard sous la forme d'un crash de NeoForge ;
//! - **idempotent** — un fichier déjà présent *et* conforme n'est pas
//!   retéléchargé. Relancer une installation interrompue reprend où elle en
//!   était, ce qui compte quand il reste 2 500 objets d'assets ;
//! - **atomique** — l'écriture passe par un `.part` renommé à la fin. Une
//!   coupure ne laisse jamais un fichier tronqué que la vérification d'un
//!   prochain passage prendrait pour valide s'il n'y avait pas d'empreinte.
mod check;
mod checksum;
mod emplacements;
mod telechargement;

pub use check::{Check, Fetched};
pub use checksum::{sha1_of_file, sha512_of_file, Checksum};
pub use emplacements::data_dir;
pub use telechargement::fichier::write_atomic;
pub use telechargement::Downloader;

/// Agent annoncé à toutes les API contactées.
///
/// Modrinth demande explicitement un agent identifiable — `projet/version
/// (contact)` — et limite plus sévèrement les agents anonymes ; Mojang et
/// Adoptium ne l'exigent pas mais le journalisent. Une seule constante pour
/// tout le launcher : c'est ce qui rend un abus traçable jusqu'à nous.
pub const USER_AGENT: &str = "samflix-mc-launcher/0.1 (+https://github.com/samflix-mc/mc-launcher)";
