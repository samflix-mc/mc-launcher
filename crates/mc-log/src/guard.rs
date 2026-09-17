//! Ce qui tient le journal ouvert aussi longtemps que le programme tourne.

mod journal;

use std::path::PathBuf;

pub use journal::log_dir;
pub(crate) use journal::{current_log_name, purge_old_logs};

/// À garder vivant aussi longtemps que le programme tourne.
///
/// Sa destruction vide la file d'écriture du fichier puis laisse à Sentry le
/// temps d'envoyer ce qui reste. Le lâcher tout de suite perdrait précisément
/// les derniers messages — ceux qui décrivent la sortie.
pub struct Guard {
    // Le fichier se vide avant que Sentry n'attende le réseau : l'ordre des
    // champs est celui des destructions. L'inverse laissait la fin du journal
    // dans la file d'écriture pendant les dix secondes d'envoi — et un Ctrl-C
    // pendant l'attente l'y laissait pour de bon.
    _file: Option<tracing_appender::non_blocking::WorkerGuard>,
    _sentry: Option<sentry::ClientInitGuard>,
    /// Répertoire du journal et nom du composant, pour retrouver le fichier du
    /// jour à la demande.
    journal: Option<(PathBuf, String)>,
}

impl Guard {
    /// Assemblé par [`crate::init`] seul, une fois les couches posées.
    ///
    /// L'ordre des arguments est celui des champs, donc celui des
    /// destructions : le fichier d'abord, Sentry ensuite.
    pub(crate) fn new(
        file: Option<tracing_appender::non_blocking::WorkerGuard>,
        sentry: Option<sentry::ClientInitGuard>,
        journal: Option<(PathBuf, String)>,
    ) -> Self {
        Self {
            _file: file,
            _sentry: sentry,
            journal,
        }
    }

    /// Un garde qui ne tient rien.
    ///
    /// C'est l'état d'une exécution dont le répertoire de journal n'était pas
    /// inscriptible : le programme tourne, mais [`log_path`] ne désigne rien.
    /// Exposé parce que les binaires reçoivent un `&Guard` en argument et que
    /// ce cas-là — celui où l'on ne doit surtout pas renvoyer l'utilisateur
    /// vers un fichier absent — ne peut être éprouvé autrement.
    ///
    /// [`log_path`]: Guard::log_path
    pub fn sans_journal() -> Self {
        Self::new(None, None, None)
    }

    /// Un garde qui désigne un journal sans rien tenir ouvert.
    ///
    /// Complément du précédent : il permet d'éprouver les deux branches de ce
    /// que [`log_path`] rend, sans poser de souscripteur global — ce que
    /// [`crate::init`] ne peut faire qu'une fois par processus.
    ///
    /// [`log_path`]: Guard::log_path
    #[doc(hidden)]
    pub fn new_pour_essais(dir: PathBuf, component: &str) -> Self {
        Self::new(None, None, Some((dir, component.to_string())))
    }

    /// Chemin du journal, à citer quand quelque chose échoue.
    ///
    /// Recalculé à chaque appel, jamais figé au démarrage : `rolling::daily`
    /// change de fichier à minuit UTC, et une partie commencée avant continue
    /// dans le suivant. Un chemin figé désignerait alors un fichier qui existe
    /// mais s'arrête avant la panne — plus trompeur qu'un fichier absent,
    /// puisque rien n'invite à en douter.
    pub fn log_path(&self) -> Option<PathBuf> {
        self.journal
            .as_ref()
            .map(|(dir, component)| dir.join(current_log_name(component)))
    }
}

#[cfg(test)]
#[path = "guard.test.rs"]
mod tests;
