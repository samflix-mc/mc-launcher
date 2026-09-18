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
pub(crate) use horodatage::now_utc;

/// Le verrou, **de la même forme que le manifeste et plus riche**.
///
/// C'est une contrainte volontaire : les deux fichiers vivent côte à côte,
/// se lisent l'un après l'autre, et se comparent du regard. Qu'ils nomment la
/// même chose autrement coûte à chaque lecture. Le nom du pack s'appelait
/// `pack` ici et `name` là ; la source d'un mod, `origin` ici et `source` là.
/// Les anciens noms restent acceptés en lecture — les verrous déjà publiés
/// n'ont pas à être réécrits pour être lus — mais ne sont plus produits.
///
/// Ce que le verrou ajoute au manifeste : la date de génération, les champs
/// résolus de chaque mod (empreintes, taille, URL, raison de sa présence), et
/// les dépendances que personne n'a su fournir.
///
/// Ce qu'il en reprend, jusqu'aux serveurs : un verrou seul suffit à installer
/// **et** à savoir où se connecter. Un outil tiers — la CI de mc-content, un
/// script de serveur — n'a plus besoin des deux fichiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub schema: u32,
    /// Nom du pack. `pack` dans les verrous d'avant.
    #[serde(alias = "pack")]
    pub name: String,
    /// Version du pack, telle que le manifeste la déclare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Date de génération, en UTC.
    pub generated: String,
    pub minecraft: String,
    pub loader: LockedLoader,
    pub java: u32,
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
    /// Où se connecter, par environnement — repris du manifeste tel quel.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub servers: std::collections::BTreeMap<String, crate::manifest::Server>,
    pub mods: Vec<LockedMod>,
    /// Dépendances qu'aucune source n'a su fournir. Vide en temps normal ;
    /// non vide, c'est le premier endroit à regarder quand le jeu refuse de
    /// démarrer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<LockedMissing>,
}
