//! Le verrou : ce qui a réellement été installé, et pourquoi.
//!
//! Le manifeste dit ce qu'on veut, le verrou dit ce qu'on a eu. L'écart entre
//! les deux est exactement ce que la résolution a décidé : la version choisie
//! quand le manifeste n'en imposait aucune, et les dépendances ajoutées
//! d'elles-mêmes.
//!
//! Il sert à deux choses, et chacune justifierait à elle seule de l'écrire :
//!
//! - **rejouer une installation à l'identique**, des mois plus tard, alors que
//!   toutes les versions ont bougé. C'est ce que fait `install --locked` ;
//! - **rendre lisible ce qui a été ajouté sans être demandé**. Six mois après,
//!   personne ne sait plus si un jar est là par choix ou parce qu'un autre
//!   l'exigeait — la ligne `reason` répond.
//!
//! Il se versionne à côté du manifeste, et une modification qu'on n'explique
//! pas dans une revue est un signal.

mod entrees;
mod horodatage;
mod lecture;
mod resume;

use serde::{Deserialize, Serialize};

pub use entrees::{LockedLoader, LockedMissing, LockedMod};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub schema: u32,
    pub pack: String,
    /// Date de génération, en UTC.
    pub generated: String,
    pub minecraft: String,
    pub loader: LockedLoader,
    pub java: u32,
    pub mods: Vec<LockedMod>,
    /// Dépendances qu'aucune source n'a su fournir. Vide en temps normal ;
    /// non vide, c'est le premier endroit à regarder quand le jeu refuse de
    /// démarrer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<LockedMissing>,
}
