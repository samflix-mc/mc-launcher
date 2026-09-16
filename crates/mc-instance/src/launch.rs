//! Construire la ligne de commande qui démarre le jeu.
//!
//! Minecraft ne se lance pas : il se *compose*. Le descripteur de NeoForge ne
//! contient qu'un delta et désigne son socle par `inheritsFrom` ; il faut
//! fusionner les deux, choisir les bibliothèques valables pour ce système,
//! assembler un classpath, puis remplacer une vingtaine de variables dans des
//! arguments dont certains n'apparaissent que sous condition.
//!
//! Quatre points décident que le jeu démarre ou non :
//!
//! - **l'ordre du classpath** — NeoForge remplace certaines bibliothèques de
//!   Mojang. Sa version doit passer devant, sinon la JVM charge celle de
//!   Mojang et le chargeur échoue sur une méthode absente ;
//! - **le client vanilla** — NeoForge ne le déclare pas parmi ses
//!   bibliothèques. Il est ajouté au classpath, et c'est FML qui le transforme
//!   au chargement ;
//! - **les natives** — inutile de les extraire : les arguments de Mojang
//!   passent `org.lwjgl.system.SharedLibraryExtractPath`, et LWJGL 3.3 sort
//!   lui-même ses binaires des jars du classpath. Il suffit que le répertoire
//!   existe ;
//! - **les drapeaux** — `--quickPlayMultiplayer` n'existe dans le descripteur
//!   que derrière une règle `is_quick_play_multiplayer`. Ignorer les règles de
//!   drapeaux produit une ligne de commande que le jeu refuse.
mod arguments;
mod chemin;
mod classpath;
mod commande;
mod descripteur;
mod execution;
mod session;
mod variables;

pub use commande::Command;
pub use execution::{run, Outcome, Report};
pub use session::{LaunchOptions, QuickPlay, Session};

pub use chemin::build;
