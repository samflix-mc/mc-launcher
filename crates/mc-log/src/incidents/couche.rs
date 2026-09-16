//! La couche qui envoie à Sentry, et ce qu'elle envoie de chaque niveau.

use tracing_subscriber::Layer;

use crate::BoxedLayer;

/// Trois traitements distincts, selon ce qu'un niveau signifie :
///
/// - `Event` ouvre un incident. Réservé aux erreurs, sans quoi le tableau de
///   bord se remplit de bruit et plus personne ne le regarde ;
/// - `Log` alimente les journaux structurés, cherchables et lisibles à côté
///   de l'incident correspondant ;
/// - `Breadcrumb` raconte ce qui a précédé, et n'est envoyé qu'attaché à un
///   incident — donc gratuit tant que rien n'échoue.
///
/// `debug` reste hors des journaux structurés : c'est le niveau du fichier,
/// des milliers de lignes par installation, et l'envoyer coûterait un quota
/// pour un détail qu'on lit de toute façon en local.
pub(crate) fn couche() -> BoxedLayer {
    sentry_tracing::layer()
        .event_filter(|meta| match *meta.level() {
            tracing::Level::ERROR => {
                sentry_tracing::EventFilter::Event | sentry_tracing::EventFilter::Log
            }
            tracing::Level::WARN | tracing::Level::INFO => {
                sentry_tracing::EventFilter::Breadcrumb | sentry_tracing::EventFilter::Log
            }
            tracing::Level::DEBUG => sentry_tracing::EventFilter::Breadcrumb,
            tracing::Level::TRACE => sentry_tracing::EventFilter::Ignore,
        })
        .boxed()
}
