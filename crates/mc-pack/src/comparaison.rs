//! « Y a-t-il quelque chose à faire avant de jouer ? »
//!
//! C'est la question que le bouton unique pose à chaque clic, et c'est la
//! seule chose que ce module sait faire.
//!
//! ## Pourquoi un module à part, et pourquoi pas `load_remote`
//!
//! Le seul chemin existant qui aille chercher le pack distant est
//! `load_remote` : il récupère le manifeste ET le verrou ensemble, puis **les
//! sauve dans le cache** avant que la moindre installation n'ait commencé
//! (`source/distant/recuperation.rs:21-45`).
//!
//! Le réutiliser ici serait un contresens. Le cache est la description du pack
//! **posé**, celle sur laquelle `jeu::coherence::verifier` se fonde ; y écrire
//! le pack **publié** avant d'avoir installé quoi que ce soit ferait croire au
//! reste du programme qu'on a déjà ce qu'on vient seulement de regarder. La
//! vérification hors-ligne qui suivrait comparerait le disque à un verrou que
//! rien n'a installé.
//!
//! D'où une fonction dédiée, qui lit et ne garde rien — et un test qui pose
//! que le répertoire de cache est **inchangé** après une comparaison. C'est le
//! seul moyen de tenir cette frontière dans la durée : elle ne se voit pas
//! dans une signature.
//!
//! ## Le geste unique
//!
//! `docs/lancement.md` posait « installer et jouer restent deux gestes ». Ce
//! module renverse la décision, et résout son motif au lieu de le contredire :
//! le motif était qu'installer coûte cher et qu'on ne doit pas le déclencher
//! par surprise. Avec une comparaison d'empreintes, JOUER ne coûte plus rien
//! quand il n'y a rien à faire — et quand il y a quelque chose à faire, ne
//! pas le faire donnerait une éjection à la connexion.
//!
//! Le CLI garde ses deux commandes : ce sont des usages d'outilleur.

use anyhow::Result;
use serde::Serialize;

use crate::etat::EtatLocal;
use crate::lockfile::Lockfile;
use crate::source::Source;

/// Ce que le bouton doit dire.
///
/// Deux valeurs et pas trois : l'ACTIVITÉ — une installation en cours, une
/// partie qui tourne — ne se déduit pas du disque, elle s'observe. La
/// confondre avec l'action ferait répondre « une installation est déjà en
/// cours » à quelqu'un qui clique pendant sa partie.
///
/// Un enum sans `Default` : c'est ce qui rend les mutants de retour
/// inviables. Un `String` ou un `bool` auraient laissé passer `String::new()`
/// et `false` sans qu'aucun test ne bronche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Rien n'est posé : le geste est une première installation.
    Installer,
    /// Quelque chose est posé : le geste est de jouer, quitte à rattraper.
    Jouer,
}

/// Ce qui sépare le posé du publié.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ecart {
    /// Rien n'est installé sur ce poste.
    Absent,
    /// Le posé correspond au publié : jouer ne téléchargera rien.
    AJour,
    /// Le publié a bougé : quelques fichiers à rattraper.
    MiseAJour,
    /// Le pack demande une réinstallation complète.
    Reinstallation,
    /// On ne sait pas — le pack publié est injoignable.
    ///
    /// **Ce n'est pas une erreur.** Un joueur hors-ligne dont le pack est
    /// cohérent doit pouvoir jouer : le bouton dit JOUER, et l'installation
    /// qui suit repartira du cache.
    Inconnu,
}

/// Ce que le front a besoin de savoir pour dessiner l'écran.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatDuPack {
    pub action: Action,
    pub ecart: Ecart,
    /// Le pack publié n'a pas répondu.
    pub hors_ligne: bool,
    /// Quelque chose est posé sur ce disque.
    pub installe: bool,
    /// Le nom du pack, tel que le verrou le porte.
    pub nom: Option<String>,
    /// Sa version, quand le pack en déclare une.
    pub version: Option<String>,
    /// La version majeure de Java que le verrou EXIGE.
    ///
    /// Lue dans le verrou et non dans une installation : la page Configuration
    /// doit pouvoir l'afficher même quand rien n'est encore installé.
    pub java: Option<u32>,
    /// Combien de mods le pack publié compte.
    pub mods: usize,
    /// La génération demandée par le pack publié.
    pub generation: u32,
}

/// Ce qui est posé sur ce disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presence {
    /// L'instance existe et le launcher a retenu ce qu'il y a mis.
    pub installe: bool,
    pub etat: Option<EtatLocal>,
}

/// Lit le disque. **Ne rend jamais d'erreur.**
///
/// Un disque qu'on ne sait pas lire veut dire « rien d'installé », et non
/// « impossible de jouer ». La distinction n'apporterait rien à l'appelant :
/// dans les deux cas la conduite est d'installer.
///
/// Corrige la troisième dette du dépôt : `Etat.installation` n'était peuplé
/// que par une installation faite dans la session courante, si bien qu'au
/// démarrage un pack parfaitement installé paraissait absent.
pub fn presence(options: &crate::Options, nom_du_pack: &str) -> Presence {
    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(nom_du_pack));

    let etat = EtatLocal::lire(&crate::etat::chemin(&instance));

    // Les deux, et pas l'un ou l'autre. Un `etat.json` sans répertoire de mods
    // décrit une installation dont quelqu'un a effacé la moitié à la main ;
    // un répertoire de mods sans état est une installation d'avant cette
    // version, dont on ne sait rien.
    let installe = etat.is_some() && instance.mods_dir().is_dir();

    Presence { installe, etat }
}

/// Va chercher le verrou publié, et **n'écrit rien**.
///
/// Ni cache, ni fichier temporaire, ni trace. Voir l'en-tête du module pour
/// la raison — elle n'est pas devinable depuis cette signature, et c'est
/// précisément pourquoi un test la garde.
pub async fn verrou_publie(url: &str, dl: &mc_dl::Downloader) -> Result<Lockfile> {
    let adresse = crate::source::lock_url_for(url);
    let octets = dl.bytes(&adresse).await?;
    Lockfile::parse(&octets)
}

/// Compare ce qui est posé à ce qui est publié.
///
/// Le cœur du bouton unique. Ne télécharge que le verrou — quelques dizaines
/// de kilooctets — et ne touche pas au disque.
pub async fn comparer(
    source: &Source,
    options: &crate::Options,
    dl: &mc_dl::Downloader,
) -> EtatDuPack {
    let url = match source {
        Source::Remote { url, .. } => url.clone(),
        // Un pack local n'a pas de « publié » : c'est le fichier qu'on est en
        // train d'éditer qui fait foi. On ne prétend pas savoir s'il y a une
        // mise à jour, et le bouton se règle sur la seule présence.
        Source::File { .. } => return sans_reseau(options, "pack local"),
    };

    let publie = match verrou_publie(&url, dl).await {
        Ok(verrou) => verrou,
        Err(erreur) => {
            tracing::warn!(url, erreur = %erreur, "verrou publié injoignable");
            return sans_reseau(options, &url);
        }
    };

    let pose = presence(options, &publie.name);
    let empreinte_publiee = publie.empreinte().ok();

    let ecart = match (&pose.etat, empreinte_publiee) {
        // Rien de posé : première installation.
        (None, _) => Ecart::Absent,
        // L'empreinte du verrou est incalculable — cas qui ne devrait pas
        // arriver, la sérialisation d'une structure qu'on vient de lire. On
        // n'invente pas : on rattrapera par différence, ce qui ne coûte qu'un
        // parcours d'empreintes.
        (Some(_), None) => Ecart::MiseAJour,
        (Some(etat), Some(empreinte)) => {
            if etat.generation < publie.generation {
                // La purge l'emporte sur tout : c'est une demande explicite de
                // celui qui publie, et elle vaut même si les verrous sont par
                // ailleurs identiques.
                Ecart::Reinstallation
            } else if etat.verrou_sha512 == empreinte {
                Ecart::AJour
            } else {
                Ecart::MiseAJour
            }
        }
    };

    EtatDuPack {
        action: if pose.installe {
            Action::Jouer
        } else {
            Action::Installer
        },
        ecart,
        hors_ligne: false,
        installe: pose.installe,
        nom: Some(publie.name.clone()),
        version: publie.version.clone(),
        java: Some(publie.java),
        mods: publie.mods.len(),
        generation: publie.generation,
    }
}

/// Ce qu'on répond quand le publié est hors de portée.
///
/// **Pas une erreur.** Le bouton dit JOUER si quelque chose est posé, et
/// l'installation qui suivra repartira du cache. Refuser de jouer parce qu'on
/// n'a pas pu vérifier punirait un joueur d'une panne de réseau.
fn sans_reseau(options: &crate::Options, pour: &str) -> EtatDuPack {
    // On ne connaît pas le nom du pack publié : on interroge le disque avec le
    // nom d'instance configuré, et à défaut avec le nom par défaut du réseau.
    let pose = presence(options, crate::etat::NOM_PAR_DEFAUT);
    tracing::info!(pour, installe = pose.installe, "comparaison sans réseau");

    EtatDuPack {
        action: if pose.installe {
            Action::Jouer
        } else {
            Action::Installer
        },
        ecart: Ecart::Inconnu,
        hors_ligne: true,
        installe: pose.installe,
        nom: None,
        version: None,
        java: None,
        mods: 0,
        generation: pose.etat.map(|e| e.generation).unwrap_or(0),
    }
}

#[cfg(test)]
#[path = "comparaison.test.rs"]
mod tests;
