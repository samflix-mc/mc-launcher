//! Plomberie partagée : téléchargement vérifié par empreinte, réessayable.
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
//! - **observable** — un gigaoctet descend en plusieurs minutes, et une
//!   fenêtre qui ne dit rien pendant ce temps-là ne se distingue pas d'une
//!   fenêtre plantée. Le corps est lu morceau par morceau, et chaque morceau
//!   est annoncé : voir [`progression`].
mod check;
mod checksum;
pub mod progression;
mod telechargement;

pub use check::{Check, Fetched};
pub use checksum::{Checksum, sha1_of_file, sha512_of_bytes, sha512_of_file};
pub use progression::{Avancement, Observateur};
pub use telechargement::fichier::{ecrire_hors_du_fil, write_atomic};
pub use telechargement::{Absent, Downloader};

/// Agent annoncé à toutes les API contactées.
///
/// Modrinth demande explicitement un agent identifiable — `projet/version
/// (contact)` — et limite plus sévèrement les agents anonymes ; Mojang et
/// Adoptium ne l'exigent pas mais le journalisent. Une seule constante pour
/// tout le launcher : c'est ce qui rend un abus traçable jusqu'à nous.
pub const USER_AGENT: &str = "samflix-mc-launcher/0.1 (+https://github.com/samflix-mc/mc-launcher)";
