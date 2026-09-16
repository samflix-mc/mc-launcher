//! « Est-ce que ça remonte vraiment ? »

use super::flush_incidents;

/// Envoie un incident de test et renvoie son identifiant.
///
/// Sert à répondre à « est-ce que ça remonte vraiment ? » sans avoir à
/// provoquer une vraie panique. L'identifiant rendu est celui à chercher dans
/// le tableau de bord : si les deux correspondent, la chaîne entière — envoi,
/// réseau, projet, censure — est vérifiée.
pub fn send_test_event() -> (sentry::types::Uuid, bool) {
    // Les deux canaux passent par des routes différentes et des filtres
    // différents : les tester ensemble évite de croire l'un fonctionnel parce
    // que l'autre l'est.
    // Vérifier une censure demande une marque que seule la nôtre produit.
    //
    // Deux tentatives ont échoué sur ce point. Un attribut contenant
    // « access_token » ressort « [Filtered] » : c'est le filtrage serveur de
    // Sentry, qui reconnaît le mot-clé. Un JWT nu ressort également
    // « [Filtered] » : Sentry reconnaît aussi la forme. Dans les deux cas le
    // test passait sans rien dire de `before_send_log`, puisque le résultat
    // aurait été le même si notre filtre n'avait pas tourné.
    //
    // Le chemin du répertoire personnel, lui, n'est un secret pour personne :
    // aucune règle serveur ne le touche. Notre filtre le réduit à « ~ ». Cette
    // réécriture-là ne peut venir que de nous.
    let temoin = std::env::var("HOME").unwrap_or_else(|_| "/home/utilisateur".into());

    tracing::info!(
        canal = "journaux structurés",
        composant = "mc-log",
        // Ces deux-là montrent la défense en profondeur, sans rien prouver.
        avec_mot_cle = "access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA",
        jeton_nu = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdEp3dE51",
        // Celui-ci tranche : « ~/… » prouve que before_send_log a tourné,
        // le chemin complet prouve qu'il n'a pas tourné.
        temoin_chemin = %format!("{temoin}/.local/share/samflix-mc"),
        "ligne de journal de test"
    );

    let id = sentry::capture_message(
        "incident de test émis par mc-pack diagnostic --incident-test",
        sentry::Level::Info,
    );
    // L'envoi est asynchrone : sans cette attente, le processus se terminerait
    // avant que la requête ne parte. Le retour dit si la file s'est vidée —
    // c'est la différence entre « un identifiant a été tiré » et « l'incident
    // est parti ».
    (id, flush_incidents(std::time::Duration::from_secs(10)))
}
