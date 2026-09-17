use super::{Identite, choisir};

/// Le choix est explicite, jamais deviné : `--pseudo` demande une session
/// hors-ligne. Un repli silencieux ferait entrer un joueur sur un serveur sous
/// une identité qu'il n'a pas choisie.
#[tokio::test]
async fn un_pseudo_donne_ouvre_une_session_hors_ligne() {
    let session = choisir(Identite::HorsLigne("Sam".into()))
        .await
        .expect("aucun réseau nécessaire");

    assert_eq!(session.name, "Sam");
    assert_eq!(session.user_type, "legacy");
    // Le jeu exige l'argument mais ne le valide pas hors ligne ; une chaîne
    // vide ferait échouer l'analyse des arguments.
    assert!(!session.token.is_empty());
    // L'UUID suit la règle du serveur vanilla : le joueur garde le même d'une
    // partie à l'autre, inventaire et permissions compris.
    assert_eq!(session.uuid, mc_auth::offline_session("Sam").profile.id);
}

#[tokio::test]
async fn le_meme_pseudo_donne_toujours_le_meme_joueur() {
    let une = choisir(Identite::HorsLigne("Sam".into())).await.unwrap();
    let autre = choisir(Identite::HorsLigne("Sam".into())).await.unwrap();
    assert_eq!(une.uuid, autre.uuid);

    let voisin = choisir(Identite::HorsLigne("Alex".into())).await.unwrap();
    assert_ne!(une.uuid, voisin.uuid);
}

/// Sans `--pseudo` et sans session enregistrée, le message doit donner les deux
/// issues : se connecter, ou jouer hors ligne.
///
/// Seul le message se vérifie. Le déclencher demanderait de garantir qu'aucune
/// session n'existe sur la machine d'essai — voir la note de [`SANS_SESSION`].
#[test]
fn le_refus_sans_session_donne_les_deux_issues() {
    let message = super::SANS_SESSION;

    assert!(message.contains("mc-auth login"), "{message}");
    assert!(message.contains("--pseudo"), "{message}");
}
