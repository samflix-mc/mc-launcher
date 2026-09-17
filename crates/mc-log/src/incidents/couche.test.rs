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

/// Donne les événements Sentry produits par ce que la fermeture journalise.
fn incidents_de(travail: impl FnOnce()) -> Vec<sentry::protocol::Event<'static>> {
    use tracing_subscriber::layer::SubscriberExt;

    sentry::test::with_captured_events(|| {
        let souscripteur = tracing_subscriber::registry().with(couche());
        tracing::subscriber::with_default(souscripteur, travail);
    })
}

/// `error!` est la seule porte d'entrée d'un incident : c'est par elle que ce
/// qui casse chez un joueur nous parvient. Un filtre qui ne l'ouvre plus ne
/// casse rien de visible — il rend seulement le tableau de bord muet.
#[test]
fn une_erreur_ouvre_un_incident() {
    let incidents = incidents_de(|| tracing::error!("le pack ne s'installe pas"));

    assert_eq!(incidents.len(), 1, "{incidents:?}");
    let message = incidents[0].message.clone().unwrap_or_default();
    assert!(message.contains("le pack ne s'installe pas"), "{message}");
}

/// Un avertissement n'ouvre pas d'incident : il laisse un fil d'Ariane, que
/// l'incident suivant emporte avec lui. C'est ce qui permet de lire ce que le
/// launcher faisait juste avant de tomber — sans lui, il reste l'erreur seule,
/// sans son contexte.
#[test]
fn un_avertissement_laisse_un_fil_d_ariane_dans_l_incident_suivant() {
    let incidents = incidents_de(|| {
        tracing::warn!("réessai du téléchargement");
        tracing::info!("installation du pack");
        tracing::error!("plus de place sur le disque");
    });

    assert_eq!(incidents.len(), 1, "un seul incident : l'erreur");
    let fils: Vec<String> = incidents[0]
        .breadcrumbs
        .iter()
        .map(|fil| fil.message.clone().unwrap_or_default())
        .collect();
    assert!(
        fils.iter().any(|m| m.contains("réessai du téléchargement")),
        "{fils:?}"
    );
    assert!(
        fils.iter().any(|m| m.contains("installation du pack")),
        "{fils:?}"
    );
}
