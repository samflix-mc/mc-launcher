//! Les nouvelles du réseau, telles que la fenêtre les montre.
//!
//! ## Ce que ce crate garantit
//!
//! **Aucun HTML n'atteint le DOM.** Le markdown des billets est analysé ici,
//! en Rust, vers des variantes fermées que le front parcourt avec un `@switch`.
//! C'est la moitié Rust de la condition à laquelle le CSP a été desserré ;
//! l'autre moitié est `pnpm invariants`, qui vérifie qu'aucun `innerHTML` ne
//! traîne côté front.
//!
//! ## Ce qu'il ne fait pas
//!
//! Il ne dérive AUCUN chemin. Le répertoire de cache lui est donné —
//! `mc-chemins` décide, et lui seul. Un crate qui calcule son propre chemin
//! est un crate de plus à faire diverger.
//!
//! ## Une remarque sur la vie privée, qui n'est pas celle qu'on croit
//!
//! Rapatrier les images plutôt que de les laisser charger par la WebView
//! n'évite PAS de se signaler à l'hôte : la contrainte « même hôte » fait que
//! Rust vient précisément d'interroger celui-là pour le fil. Ce que le
//! rapatriement évite est plus étroit, et vaut d'être dit exactement : l'hôte
//! ne compte plus les OUVERTURES DE LA PAGE, puisqu'une image déjà en cache
//! n'est pas redemandée.
//!
//! Ce qu'il achète vraiment, c'est le contrôle : une taille bornée, un type
//! déduit des octets, et `img-src` qui n'a pas besoin de s'ouvrir à un hôte
//! distant.

pub mod analyse;
pub mod arbre;
pub mod contrat;
pub mod liens;
pub mod recuperation;

pub use arbre::{Bloc, Inline};
pub use contrat::{Billet, Fil, SCHEMA};
pub use recuperation::{IMAGE_MAX, IMAGES_MAX, charger, en_data_url, rapatrier, url_du_fil};
