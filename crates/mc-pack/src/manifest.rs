//! Le manifeste : ce qu'un pack est, écrit une fois et versionné.
//!
//! Un seul fichier décrit l'installation entière — version du jeu, chargeur,
//! version de Java, liste des mods. Il est volontairement court : tout ce qui
//! peut être déduit l'est, et ce qui est écrit à la main est ce qu'un humain a
//! décidé.
//!
//! Chaque mod peut être laissé libre (« la dernière version compatible ») ou
//! **épinglé** sur un build précis. Les deux ont leur place : le pack de
//! développement suit les mises à jour, celui de production ne bouge que
//! lorsqu'on le décide. C'est le rôle de `file`, qui désigne un build exact.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod controle;
mod entrees;
#[cfg(test)]
mod essais;
mod lecture;
mod serveur;
mod serveurs;

pub use entrees::{Loader, ModEntry};
pub use serveur::Server;

/// Version du format. Un manifeste d'une autre version se refuse plutôt que de
/// se lire à moitié.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Version de Minecraft, p. ex. `1.21.1`.
    pub minecraft: String,
    pub loader: Loader,
    /// Version majeure de Java. À défaut, celle qu'exige Mojang pour cette
    /// version du jeu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<u32>,
    /// Le numéro de génération de l'installation.
    ///
    /// Le mécanisme par lequel celui qui publie le pack peut dire : « ne
    /// rattrape pas cette mise à jour par différence, efface et recommence ».
    ///
    /// Le fonctionnement normal est différentiel — on compare les empreintes et
    /// l'on ne retélécharge que ce qui a changé. C'est ce qu'il faut : un
    /// modpack de trois cents mods pèse un demi-gigaoctet, et le retélécharger
    /// à chaque mise à jour serait insupportable.
    ///
    /// Mais certaines transitions ne se rattrapent pas ainsi. Un mod renommé
    /// laisse son ancien jar en place, un dossier de configuration change de
    /// forme, un shader laisse des résidus que rien ne référence plus. Le
    /// différentiel ne voit que ce que le verrou décrit ; il est aveugle à ce
    /// que le verrou ne décrit PLUS.
    ///
    /// Incrémenter ce nombre déclenche alors une purge avant l'installation.
    /// Ce qui est effacé, c'est ce que le launcher a posé — mods, shaders,
    /// resource packs. **Jamais l'instance de jeu** : ni les sauvegardes, ni
    /// les options, ni les configurations que le joueur a modifiées. Perdre un
    /// monde pour rattraper un renommage de mod serait un remède pire que le
    /// mal.
    ///
    /// `#[serde(default)]` sans `skip_serializing_if` : les fichiers déjà
    /// publiés se lisent en génération 0, et tout fichier écrit désormais porte
    /// la sienne explicitement. Un champ absent à l'écriture obligerait à
    /// distinguer « jamais posé » de « posé à zéro », alors que les deux
    /// veulent dire la même chose.
    #[serde(default)]
    pub generation: u32,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
    /// Où se connecter, par environnement.
    ///
    /// Le manifeste est le même partout : c'est la même image de contenu,
    /// servie sous trois noms. Ce n'est donc pas lui qui peut choisir, c'est le
    /// client — avec son propre environnement, celui que la CI lui a figé à la
    /// compilation. Un launcher de dev rejoint le serveur de dev.
    ///
    /// Les clés sont celles de `mc_log::Environment` : `development`,
    /// `preproduction`, `production`. Une absence n'est pas une erreur — la
    /// préproduction n'a pas de serveurs Minecraft derrière elle, et le jeu s'y
    /// lance sans rejoindre quoi que ce soit.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub servers: BTreeMap<String, Server>,
}
