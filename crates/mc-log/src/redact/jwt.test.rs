use crate::redact::{MASK, redact};

#[test]
fn un_jwt_est_masque_meme_sans_mot_cle() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk";
    let sortie = redact(&format!("échec avec {jwt} en tête"));
    assert!(!sortie.contains("eyJhbGci"));
    assert!(sortie.contains(MASK));
    assert!(sortie.contains("échec avec"));
}

#[test]
fn une_empreinte_n_est_pas_prise_pour_un_secret() {
    // Les SHA-1 sont utiles au diagnostic et ne révèlent rien.
    let texte = "empreinte 88ee316e68900080b017f60c12162e2731924cf8 attendue";
    assert_eq!(redact(texte), texte);
}

#[test]
fn un_identifiant_court_commencant_par_ey_survit() {
    // « eyZ2YBGT » est un identifiant de version Modrinth, pas un jeton.
    let texte = "build épinglé eyZ2YBGT introuvable";
    assert_eq!(redact(texte), texte);
}

/// C'est la **longueur** du mot qui décide, et non l'endroit où il tombe dans
/// la ligne. Un identifiant court qui commence par « eyJ » — le préfixe exact
/// d'un JWT — doit survivre aussi loin soit-il du début : dans le cas
/// contraire, la censure dépendrait de ce qui a été écrit avant, et un même
/// message serait masqué ou non selon la longueur de son préambule.
#[test]
fn un_court_eyj_ne_se_masque_pas_parce_qu_il_arrive_tard() {
    let texte = "configuration lue depuis eyJcfg42";
    assert_eq!(redact(texte), texte);
}
