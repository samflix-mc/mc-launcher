use super::Session;

#[test]
fn une_session_hors_ligne_porte_un_jeton_non_vide() {
    // Le jeu exige l'argument ; une chaîne vide casse l'analyse.
    let session = Session::offline("Sam", "uuid");
    assert!(!session.token.is_empty());
    assert_eq!(session.user_type, "legacy");
}
