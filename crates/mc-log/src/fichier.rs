//! La couche fichier : le détail complet, censuré, avec rotation.

use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, Layer};

use crate::BoxedLayer;
use crate::guard::{log_dir, purge_old_logs};
use crate::redact::redact;

/// Écrivain qui censure chaque ligne avant de l'écrire.
///
/// Le fichier de journal est exactement ce qu'on demande à un joueur de joindre
/// quand quelque chose échoue — sur un salon Discord, dans un ticket. S'il
/// contient son jeton Microsoft, on a créé le problème qu'on voulait éviter.
/// La censure s'applique donc au fichier comme à Sentry, et pas seulement à ce
/// qui part sur le réseau.
pub(crate) struct RedactingWriter<W> {
    inner: W,
}

impl<W: std::io::Write> std::io::Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // `tracing` remet un événement formaté entier par appel : la censure
        // voit donc des lignes complètes, jamais un jeton coupé en deux.
        let text = String::from_utf8_lossy(buf);
        self.inner.write_all(redact(&text).as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub(crate) struct Redacting<M>(pub(crate) M);

impl<'a, M> tracing_subscriber::fmt::MakeWriter<'a> for Redacting<M>
where
    M: tracing_subscriber::fmt::MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter {
            inner: self.0.make_writer(),
        }
    }
}

/// Couche fichier, avec rotation quotidienne.
///
/// Un échec d'ouverture ne doit pas empêcher le programme de tourner : le
/// répertoire peut être en lecture seule ou plein. On perd le journal, pas
/// l'installation.
///
/// Rend le répertoire et non le fichier : c'est l'appender qui décide du nom du
/// jour, et il en change à minuit.
pub(crate) fn file_layer(
    component: &str,
) -> (
    Option<BoxedLayer>,
    Option<tracing_appender::non_blocking::WorkerGuard>,
    Option<PathBuf>,
) {
    let dir = log_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return (None, None, None);
    }
    purge_old_logs(&dir);

    let appender = tracing_appender::rolling::daily(&dir, format!("{component}.log"));
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::layer()
        .with_writer(Redacting(writer))
        // Un fichier relu plus tard, souvent par quelqu'un d'autre : pas de
        // couleurs, et la cible du message est nécessaire pour situer.
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("debug,hyper=info,reqwest=info,rustls=info"))
        .boxed();

    (Some(layer), Some(guard), Some(dir))
}

#[cfg(test)]
#[path = "fichier.test.rs"]
mod tests;
