//! D'où vient le pack : un fichier du dépôt, ou une adresse.
//!
//! Les deux cas ne servent pas les mêmes gens, et c'est ce qui décide de leur
//! comportement.
//!
//! **Un fichier** est ce qu'on édite. Le manifeste dit ce qu'on veut, la
//! résolution cherche les versions, et le verrou est réécrit à côté. C'est le
//! geste de développement, celui qui fait bouger le pack.
//!
//! **Une adresse** est ce qu'on reçoit. Le manifeste et son verrou sont
//! téléchargés ensemble, et le verrou est **rejoué tel quel** : un joueur ne
//! résout rien. S'il le faisait, sa machine choisirait ses propres versions le
//! jour où un mod en publie une nouvelle, et il arriverait sur le serveur avec
//! des registres NeoForge qui ne concordent plus — une éjection à la connexion,
//! sans message utile.
//!
//! Le verrou distant est donc la seule source de vérité côté joueur, et c'est
//! exactement ce que mc-content publie.
//!
//! ## Hors-ligne
//!
//! Chaque téléchargement réussi laisse une copie dans le cache. Quand le réseau
//! manque, cette copie est reprise et l'utilisateur en est averti : jouer avec
//! le pack d'hier vaut mieux que ne pas jouer. Rien n'est mis en cache avant
//! d'avoir été relu — une réponse tronquée ou une page d'erreur HTML
//! remplaceraient sinon un pack valide par du vide.

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use std::path::PathBuf;

/// Adresse du pack en production. mc-launcher-site sert le répertoire
/// `launcher/` de l'image mc-content, qui est l'endroit où la liste des mods
mod cache;
mod distant;
mod lecture;
mod local;

pub use distant::{URL_DEVELOPPEMENT, URL_PREPRODUCTION, URL_PRODUCTION, url_par_defaut};

#[derive(Debug, Clone)]
pub enum Source {
    /// Un chemin sur le disque. Le verrou se trouve à côté, et sera réécrit.
    File { manifest: PathBuf },
    /// Une URL. Le manifeste et le verrou sont téléchargés puis mis en cache.
    Remote { url: String, cache_dir: PathBuf },
}

/// Un pack lu, quelle qu'en soit la provenance.
pub struct Pack {
    pub manifest: Manifest,
    /// Verrou déjà connu : celui du dépôt, ou celui qui vient d'être
    /// téléchargé. Absent la première fois qu'un pack local est résolu.
    pub lock: Option<Lockfile>,
    /// Où écrire le verrou, quand il y a lieu de l'écrire.
    pub lock_path: PathBuf,
    /// Le verrou fait foi : ses builds sont rejoués au lieu d'être cherchés.
    pub replay: bool,
    /// Le réseau a manqué et le cache a pris le relais.
    pub from_cache: bool,
}
