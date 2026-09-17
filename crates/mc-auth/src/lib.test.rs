use super::{Profile, Session, offline_session};

/// Le jeton vide est la seule différence visible d'ici entre une session
/// Microsoft et une session locale — et c'est elle qui décide si un serveur en
/// ligne acceptera le joueur.
#[test]
fn une_session_sans_jeton_n_ouvre_pas_de_serveur_en_ligne() {
    assert!(!offline_session("Sam").est_en_ligne());

    let en_ligne = Session {
        minecraft_token: "jeton-msa".into(),
        profile: Profile {
            id: "0123".into(),
            name: "Sam".into(),
        },
    };
    assert!(en_ligne.est_en_ligne());
}

/// L'UUID est en hexadécimal sans tirets : c'est la forme que le jeu attend
/// sur sa ligne de commande.
#[test]
fn l_uuid_annonce_au_jeu_n_a_pas_de_tirets() {
    let session = offline_session("Sam");
    assert_eq!(session.profile.id.len(), 32, "{}", session.profile.id);
    assert!(!session.profile.id.contains('-'));
    assert!(session.profile.id.chars().all(|c| c.is_ascii_hexdigit()));
}
