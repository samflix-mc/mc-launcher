use super::couche;

/// La couche s'assemble, et accepte les cinq niveaux sans paniquer.
///
/// Le filtre d'événements est une fermeture : elle ne s'appelle qu'au passage
/// d'un événement. Sans souscripteur posé, elle ne tournerait jamais — et
/// c'est justement elle qui décide ce qui ouvre un incident.
///
/// Aucun client Sentry n'est initialisé ici : `sentry_tracing` classe les
/// événements, mais rien ne part sur le réseau.
#[test]
fn chaque_niveau_traverse_la_couche_sans_incident() {
    use tracing_subscriber::layer::SubscriberExt;

    let souscripteur = tracing_subscriber::registry().with(couche());
    tracing::subscriber::with_default(souscripteur, || {
        // `error` ouvre un incident, `warn` et `info` laissent un fil
        // d'Ariane, `debug` aussi, `trace` est ignoré.
        tracing::error!("une erreur");
        tracing::warn!("un avertissement");
        tracing::info!("un jalon");
        tracing::debug!("un détail");
        tracing::trace!("le dernier recours");
    });
}
