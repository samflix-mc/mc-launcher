//! Installation d'une instance Minecraft : fichiers Mojang, puis NeoForge.
//!
//! Le disque est organisé comme celui des launchers multi-instances, pour une
//! raison de place : les bibliothèques et les assets d'une version pèsent près
//! d'un gigaoctet et ne dépendent que de la version du jeu. Ils sont partagés,
//! et seules les données propres à une partie — mods, configuration,
//! sauvegardes — appartiennent à l'instance.
//!
//! ```text
//! <données>/
//!   shared/                  ← joue le rôle d'un .minecraft
//!     versions/1.21.1/        descripteur et client vanilla
//!     versions/neoforge-21.1.250/
//!     libraries/  assets/
//!   instances/<nom>/
//!     minecraft/              répertoire de jeu : mods, config, saves
//!   runtime/temurin-21/
//! ```
//!
//! Le répertoire `shared` a la forme d'un `.minecraft` parce que
//! l'installateur NeoForge l'exige : il y cherche le client vanilla à patcher
//! et y dépose ce qu'il fabrique.

mod disposition;
mod verification;

#[cfg(test)]
mod essais;

pub mod crash;
pub mod launch;
pub mod neoforge;
pub mod vanilla;

pub use disposition::{Instance, Layout};
pub use verification::verify;
