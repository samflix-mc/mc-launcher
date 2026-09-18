//! Où le launcher range ce qu'il installe, ce qu'il configure et ce qu'il jette.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Le nom sous lequel le launcher apparaît dans les répertoires du système.
///
/// **L'identifiant de l'application, et non le nom du réseau.** C'est
/// exactement ce que `app_data_dir()` et `app_config_dir()` de Tauri
/// composent : les deux moitiés du programme — la fenêtre, qui interroge le
/// résolveur, et la ligne de commande, qui dérive de l'environnement —
/// aboutissent donc au même répertoire par deux chemins différents.
///
/// Ce segment valait `samflix-mc` jusqu'au 18 septembre 2026, et l'argument
/// était bon : ne pas abandonner ce qui était déjà posé. Il a été renversé
/// délibérément, pour une raison plus forte — un launcher qui range ses
/// affaires ailleurs que là où son propre framework les attend est un piège
/// qui se redécouvre à chaque lecture du code, et le doute revient à chaque
/// fois. Le coût du renversement est payé UNE fois, par un déplacement de
/// répertoire ; celui du doute se paie à chaque passage.
///
/// **Ce qui n'a PAS changé** : le service du trousseau s'appelle toujours
/// `samflix-mc`. Ce n'est pas un chemin, c'est une clé du gestionnaire de
/// secrets — la renommer déconnecterait les sessions ouvertes sans rien
/// apporter.
pub const SEGMENT: &str = "mc.samflix.launcher";

/// Les quatre racines dont le launcher a besoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emplacements {
    /// Ce qui est gros et reconstructible : instances, runtimes Java, caches.
    pub donnees: PathBuf,
    /// Ce qui est petit et précieux : la session, les réglages.
    pub config: PathBuf,
    /// Les journaux du jour et des jours précédents.
    pub journaux: PathBuf,
    /// Ce qui ne survit pas à un redémarrage : écritures en cours.
    pub temporaire: PathBuf,
}

/// Les racines brutes, avant que le segment du launcher n'y soit ajouté.
///
/// Séparé d'[`Emplacements`] pour que la DÉDUCTION — qui ne peut pas diverger
/// d'une plateforme à l'autre — soit distincte du CHOIX des racines, qui, lui,
/// diverge nécessairement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bases {
    pub donnees: PathBuf,
    pub config: PathBuf,
    pub temporaire: PathBuf,
}

/// Ce qui se déduit des racines, et rien d'autre.
///
/// Que des `join`. C'est la seule partie dont on puisse affirmer qu'elle ne
/// varie pas selon la plateforme, et c'est pour cela qu'elle est séparée : le
/// `chemins.rs` de l'application compare ce que Tauri lui donne à ce que
/// [`du_systeme`] trouve, et cette comparaison n'aurait aucun sens si la
/// déduction elle-même pouvait différer.
pub fn depuis_bases(bases: Bases) -> Emplacements {
    Emplacements {
        // `<donnees>/logs`, et NON un « app_log_dir » du système. Sous macOS,
        // celui-ci range sous `~/Library/Logs`, c'est-à-dire HORS des données :
        // supprimer le répertoire de données ne suffirait plus à repartir de
        // zéro, ce qui est la seule promesse que cette arborescence fait.
        journaux: bases.donnees.join("logs"),
        donnees: bases.donnees,
        config: bases.config,
        temporaire: bases.temporaire,
    }
}

/// Ce que le système dit, plateforme par plateforme.
///
/// La fonction choisit la branche ; les trois fonctions qu'elle appelle sont
/// compilées PARTOUT et n'ont aucun `cfg`. C'est ce qui permet d'éprouver la
/// branche Windows depuis un runner Linux : sans cela, deux branches sur trois
/// ne seraient jamais exécutées par la CI, et chacune rendrait des mutants
/// survivants indéfiniment.
pub fn du_systeme() -> Emplacements {
    let lire = |cle: &str| std::env::var_os(cle);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let temp = std::env::temp_dir();

    let bases = if cfg!(windows) {
        bases_windows(&lire, temp)
    } else if cfg!(target_os = "macos") {
        bases_macos(&lire, home, temp)
    } else {
        bases_linux(&lire, home, temp)
    };

    depuis_bases(bases)
}

/// La convention XDG, et ses deux pièges.
///
/// Une variable POSÉE MAIS VIDE ne désigne rien : la traiter comme une racine
/// mettrait les données à `/samflix-mc`, à la racine du disque. Et un `HOME`
/// absent — un service, un conteneur — ne doit pas faire paniquer : on retombe
/// sur le répertoire courant, ce qui est mauvais mais visible, là où une
/// panique serait muette dans une application graphique.
pub fn bases_linux(
    lire: &impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
    temporaire: PathBuf,
) -> Bases {
    let racine = |variable: &str, defaut: &[&str]| -> PathBuf {
        match lire(variable).filter(|valeur| !valeur.is_empty()) {
            Some(valeur) => PathBuf::from(valeur).join(SEGMENT),
            None => defaut
                .iter()
                .fold(courant_ou(home.clone()), |chemin, part| chemin.join(part))
                .join(SEGMENT),
        }
    };

    Bases {
        donnees: racine("XDG_DATA_HOME", &[".local", "share"]),
        // La configuration N'EST PAS sous les données, et c'est la seule
        // plateforme où la distinction existe réellement : c'est ce qui permet
        // de supprimer huit cents mégaoctets d'instances sans perdre la
        // session ni les réglages.
        config: racine("XDG_CONFIG_HOME", &[".config"]),
        temporaire: temporaire.join(SEGMENT),
    }
}

/// macOS range tout sous `Application Support`.
///
/// Données et configuration au MÊME endroit, et ce n'est pas un raccourci :
/// c'est ce que fait le résolveur de Tauri (`path/desktop.rs:62-63,73-74`), et
/// s'en écarter ferait diverger l'application de ses propres crates. La
/// conséquence est à écrire dans la documentation plutôt qu'à corriger ici :
/// la promesse « supprimer les données sans perdre les préférences » ne vaut
/// que sous Linux.
pub fn bases_macos(
    lire: &impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
    temporaire: PathBuf,
) -> Bases {
    let _ = lire;
    let support = courant_ou(home)
        .join("Library")
        .join("Application Support")
        .join(SEGMENT);

    Bases {
        donnees: support.clone(),
        config: support,
        temporaire: temporaire.join(SEGMENT),
    }
}

/// Windows range tout sous `APPDATA`, l'itinérant.
///
/// `APPDATA` et non `LOCALAPPDATA` : c'est ce que rend le résolveur de Tauri
/// pour les deux, et un profil itinérant y emporte donc les instances avec
/// lui. C'est discutable pour huit cents mégaoctets ; ce n'est pas à ce crate
/// d'en décider seul, puisque l'application, elle, suivra Tauri de toute
/// façon — et deux emplacements différents seraient bien pires qu'un seul
/// mauvais.
pub fn bases_windows(lire: &impl Fn(&str) -> Option<OsString>, temporaire: PathBuf) -> Bases {
    let base = match lire("APPDATA").filter(|valeur| !valeur.is_empty()) {
        Some(valeur) => PathBuf::from(valeur).join(SEGMENT),
        // Sans APPDATA il n'y a pas de profil utilisateur : le répertoire
        // courant est le seul repli qui ne demande rien à personne.
        None => PathBuf::from(".").join(SEGMENT),
    };

    Bases {
        donnees: base.clone(),
        config: base,
        temporaire: temporaire.join(SEGMENT),
    }
}

/// Le répertoire personnel, ou le répertoire courant à défaut.
fn courant_ou(home: Option<PathBuf>) -> PathBuf {
    home.unwrap_or_else(|| PathBuf::from("."))
}

impl Emplacements {
    /// Crée les quatre répertoires s'ils n'existent pas.
    ///
    /// Appelée au démarrage plutôt qu'au premier écrit : un joueur dont le
    /// disque est plein doit l'apprendre avant d'avoir téléchargé quatre cents
    /// mégaoctets, pas après.
    pub fn creer(&self) -> std::io::Result<()> {
        for chemin in [
            &self.donnees,
            &self.config,
            &self.journaux,
            &self.temporaire,
        ] {
            std::fs::create_dir_all(chemin)?;
        }
        Ok(())
    }

    /// Les quatre racines, nommées, pour le diagnostic.
    pub fn enumerer(&self) -> [(&'static str, &Path); 4] {
        [
            ("données", self.donnees.as_path()),
            ("config", self.config.as_path()),
            ("journaux", self.journaux.as_path()),
            ("temporaire", self.temporaire.as_path()),
        ]
    }
}

#[cfg(test)]
#[path = "emplacements.test.rs"]
mod tests;
