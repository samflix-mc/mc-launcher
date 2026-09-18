//! Ce que le launcher sait de l'installation posée sur CE poste.
//!
//! ## La question à laquelle ce module répond
//!
//! Le verrou dit ce que le pack DEVRAIT être. Il ne dit pas ce qui est
//! réellement posé sur ce disque-ci, ni ce qui s'y est passé la dernière fois.
//! Jusqu'ici, le launcher ne le savait pas : `Etat.installation` n'était peuplé
//! que par une installation faite dans la session courante, si bien qu'au
//! démarrage, un pack parfaitement installé paraissait absent.
//!
//! Un fichier à côté de l'instance suffit, et c'est ce module.
//!
//! ## Pourquoi il contrôle son propre `schema`
//!
//! Le verrou porte un champ `schema` qui est écrit et jamais relu — la seule
//! lecture d'un `schema` dans tout le dépôt est celle du manifeste. Reconduire
//! ce champ mort dans un fichier neuf serait faire exprès l'erreur qu'on
//! reproche.
//!
//! Il est donc relu, et un refus se traite comme une ABSENCE : génération 0,
//! donc purge, donc réinstallation propre. C'est le comportement sûr — un
//! fichier d'état qu'on ne sait pas lire ne doit pas empêcher de jouer, et il
//! ne doit pas non plus faire croire qu'on connaît l'installation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Version du format de l'état local.
pub const SCHEMA: u32 = 1;

/// Le nom du pack quand on ne l'a pas encore lu.
///
/// Sert au seul cas où le verrou publié est injoignable : il faut bien
/// interroger un répertoire d'instance pour savoir si quelque chose est posé,
/// et l'on n'a alors aucune source qui donne le nom. C'est celui que
/// mc-content publie depuis toujours.
pub const NOM_PAR_DEFAUT: &str = "samflix";

/// Ce qu'on a retenu de la dernière installation réussie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatLocal {
    pub schema: u32,
    /// L'empreinte du verrou tel qu'il était à la fin de l'installation.
    ///
    /// C'est à elle qu'on oppose l'empreinte du verrou publié pour décider
    /// s'il y a quelque chose à faire. Sur la forme canonique des deux côtés :
    /// voir `Lockfile::empreinte`.
    pub verrou_sha512: String,
    /// La génération sous laquelle cette installation a été posée.
    pub generation: u32,
    /// Quand, en UTC. Pour le diagnostic seulement — rien ne décide d'après.
    pub pose_le: String,
}

/// Où l'état d'une instance se range.
///
/// À côté du répertoire de jeu et non dedans : ce qui est dans `minecraft/`
/// appartient au jeu, et un fichier étranger y serait ramassé par un outil
/// qui synchronise les instances.
pub fn chemin(instance: &mc_instance::Instance) -> PathBuf {
    instance.dir.join("etat.json")
}

impl EtatLocal {
    /// L'état d'une installation qu'on vient de finir.
    pub fn neuf(verrou_sha512: String, generation: u32, pose_le: String) -> Self {
        Self {
            schema: SCHEMA,
            verrou_sha512,
            generation,
            pose_le,
        }
    }

    /// Relit l'état, ou rend `None`.
    ///
    /// **Ne rend jamais d'erreur.** Un état absent, illisible, ou d'un schéma
    /// inconnu veulent tous dire la même chose : « on ne sait pas ce qui est
    /// posé ». Distinguer ces cas obligerait l'appelant à décider quoi faire
    /// d'une distinction dont il ne peut rien tirer — et la seule conduite
    /// sûre est la même dans les trois cas.
    pub fn lire(chemin: &Path) -> Option<Self> {
        let brut = std::fs::read(chemin).ok()?;
        match serde_json::from_slice::<Self>(&brut) {
            Ok(etat) if etat.schema == SCHEMA => Some(etat),
            Ok(etat) => {
                tracing::warn!(
                    schema = etat.schema,
                    attendu = SCHEMA,
                    fichier = %chemin.display(),
                    "état local d'un schéma inconnu : traité comme absent"
                );
                None
            }
            Err(erreur) => {
                tracing::warn!(
                    erreur = %erreur,
                    fichier = %chemin.display(),
                    "état local illisible : traité comme absent"
                );
                None
            }
        }
    }

    pub fn ecrire(&self, chemin: &Path) -> anyhow::Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        if let Some(parent) = chemin.parent() {
            std::fs::create_dir_all(parent)?;
        }
        mc_dl::write_atomic(chemin, json.as_bytes())
    }
}

/// Ce qu'il faut faire avant d'installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Avant {
    /// Installer par différence, comme d'habitude.
    Differentiel,
    /// Effacer ce que le launcher a posé, puis installer.
    Purger,
}

/// Faut-il purger avant d'installer ?
///
/// Fonction PURE, et c'est tout l'intérêt : la règle se lit en quatre lignes
/// et s'éprouve sans disque.
///
/// ## Les quatre cas, et pourquoi c'est `<` et non `!=`
///
/// - **Pas d'état** : on ne sait rien de ce qui est posé. Purger. C'est le cas
///   de tout poste déjà installé au premier lancement d'après cette version —
///   cent pour cent de la population, une fois.
/// - **Génération égale** : rien de spécial, différentiel.
/// - **Génération demandée SUPÉRIEURE** : celui qui publie a demandé une
///   réinstallation propre. Purger.
/// - **Génération demandée INFÉRIEURE** : un retour en arrière du pack publié.
///   On ne purge PAS. Le `<` est ici et non un `!=` pour cette raison précise :
///   revenir à une génération antérieure veut dire qu'on republie un état
///   qu'on savait bon, et effacer l'installation à cette occasion punirait le
///   joueur d'une décision d'exploitation. Le différentiel remettra les
///   fichiers d'avant.
pub fn decider(pose: Option<&EtatLocal>, generation_demandee: u32) -> Avant {
    match pose {
        None => Avant::Purger,
        Some(etat) if etat.generation < generation_demandee => Avant::Purger,
        Some(_) => Avant::Differentiel,
    }
}

/// Ce que la purge a effacé, pour le compte rendu.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Purge {
    /// Les répertoires réellement vidés, relatifs au répertoire de jeu.
    pub vides: Vec<String>,
    /// Ce qui n'a pas pu être effacé, avec la raison.
    pub echecs: Vec<String>,
}

impl Purge {
    pub fn a_eu_lieu(&self) -> bool {
        !self.vides.is_empty() || !self.echecs.is_empty()
    }
}

/// Ce que le launcher a le droit d'effacer — et RIEN d'autre.
///
/// ## Une liste blanche, et pourquoi c'est vital
///
/// Une liste NOIRE — « efface tout sauf `saves` et `options.txt` » — serait la
/// bonne façon de perdre un monde le jour où un mod range ses données dans un
/// répertoire auquel personne n'a pensé. Les journaux de serveur, les captures
/// d'écran, les schémas de Litematica, les carnets de Waystones : tout cela
/// vit dans le répertoire de jeu, et rien ne les distingue d'un résidu.
///
/// On nomme donc ce qu'on efface. Ces quatre répertoires ont une propriété
/// commune : leur contenu est INTÉGRALEMENT reposé par l'installation qui
/// suit, et le joueur n'y met jamais rien qu'il tienne à garder.
///
/// `config/` n'en fait PAS partie, et c'est le point qui demande le plus
/// d'attention : c'est là que le joueur règle ses mods, et un réglage perdu
/// est une soirée de reconfiguration. Un changement de forme de configuration
/// qui exigerait vraiment de vider `config/` demande une décision humaine,
/// annoncée aux joueurs — pas un numéro incrémenté dans un fichier.
const EFFACABLES: &[&str] = &["mods", "shaderpacks", "resourcepacks", "libraries"];

/// Efface ce que le launcher a posé, et rien de plus.
///
/// Prend le répertoire de JEU — celui qui contient `mods`, `saves`,
/// `options.txt` — et n'en vide que les répertoires de [`EFFACABLES`].
///
/// Ne rend pas d'erreur : un répertoire qu'on ne peut pas effacer — un fichier
/// verrouillé par un antivirus, un point de montage — ne doit pas empêcher
/// l'installation de continuer. Elle réécrira par-dessus, et le compte rendu
/// dira ce qui a résisté.
pub fn purger(repertoire_de_jeu: &Path) -> Purge {
    let mut purge = Purge::default();

    for nom in EFFACABLES {
        let cible = repertoire_de_jeu.join(nom);
        if !cible.exists() {
            continue;
        }
        match std::fs::remove_dir_all(&cible) {
            Ok(()) => {
                tracing::info!(repertoire = %cible.display(), "purgé");
                purge.vides.push((*nom).to_string());
            }
            Err(erreur) => {
                tracing::warn!(
                    repertoire = %cible.display(),
                    erreur = %erreur,
                    "purge impossible : l'installation écrira par-dessus"
                );
                purge.echecs.push(format!("{nom} : {erreur}"));
            }
        }
    }

    purge
}

#[cfg(test)]
#[path = "etat.test.rs"]
mod tests;
