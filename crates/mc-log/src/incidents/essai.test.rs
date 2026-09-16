use super::send_test_event;

/// Sans client Sentry, rien ne part — et c'est ce que le retour doit dire.
///
/// La nuance compte : « un identifiant a été tiré » n'est pas « l'incident est
/// parti ». Un diagnostic qui annoncerait un identifiant introuvable dans le
/// tableau de bord ferait chercher pour rien.
#[test]
fn sans_client_rien_n_est_transmis() {
    let (_id, transmis) = send_test_event();
    assert!(
        !transmis,
        "la file est dite vidée alors qu'aucun client n'existe"
    );
}
