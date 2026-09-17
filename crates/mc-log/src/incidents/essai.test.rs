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

/// Et avec un client, l'incident part, sous l'identifiant annoncé. C'est ce
/// couple-là que le diagnostic affiche : un identifiant nul, ou un « parti »
/// qui ne l'est pas, enverrait chercher dans le tableau de bord quelque chose
/// qui n'y est pas.
#[test]
fn avec_un_client_l_incident_part_sous_l_identifiant_annonce() {
    let mut resultat = None;
    let evenements = sentry::test::with_captured_events(|| {
        resultat = Some(send_test_event());
    });

    let (identifiant, transmis) = resultat.expect("send_test_event a été appelé");
    assert!(transmis, "la file n'est pas dite vidée");
    assert_ne!(
        identifiant,
        sentry::types::Uuid::nil(),
        "aucun identifiant à chercher"
    );

    // Les deux canaux : le message, et la ligne de journal structurée.
    assert_eq!(evenements.len(), 1, "{evenements:?}");
    assert_eq!(evenements[0].event_id, identifiant);
}
