//! Lancer une partie sur ce qui est installé.
//!
//! Le pendant de [`install`](crate::install) : l'un pose le pack, l'autre le
//! joue. Les deux restent des gestes distincts — voir `docs/lancement.md` : les
//! enchaîner ferait attendre huit cents mégaoctets à qui voulait seulement
//! jouer.
//!
//! ## Pourquoi c'est ici et non dans un binaire
//!
//! Ce module a longtemps vécu dans `mc-pack` **le binaire**, sous
//! `commandes/lancement/`. Tant qu'il n'y avait qu'une ligne de commande, cela
//! ne coûtait rien. Dès qu'une fenêtre a voulu lancer le jeu, elle n'a pas pu
//! l'appeler : elle a dû réassembler elle-même la session, le Java du verrou et
//! la ligne de commande. Deux assemblages pour la même chose, dont l'un
//! seulement vérifie la cohérence de l'instance — c'est exactement la
//! divergence qu'on ne veut pas.
//!
//! Il est donc dans la bibliothèque, et les deux appelants s'en servent. Ce qui
//! reste au binaire est ce qui lui appartient vraiment : l'affichage.
//!
//! ## Ce que la préparation vérifie avant de lancer
//!
//! Le pack **local**, jamais celui du jour : aller chercher le pack publié
//! décrirait des mods que le dossier ne contient pas, et interdirait de jouer
//! sans réseau. Le Java **du verrou**, pas celui du système : c'est avec lui
//! que NeoForge a été installé. Et la cohérence de l'instance, sans quoi le
//! serveur tranche par une éjection qui ne nomme pas sa cause.

mod cible;
mod coherence;
mod enchainement;
mod execution;
mod identite;
pub mod incidents;
mod instance;
mod preparation;

pub use enchainement::{Deroulement, doit_rattraper, mettre_a_jour, mettre_a_jour_et_jouer};
pub use execution::{jouer, journaux};
pub use identite::Identite;
pub use preparation::{Confort, Partie, preparer};
