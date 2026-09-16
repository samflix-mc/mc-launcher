use super::*;

#[test]
fn sans_declaration_on_reste_local() {
    // Le cas d'un `cargo run --release` sur un poste : ce n'est pas parce
    // que le profil est « release » que le déploiement est en production.
    assert_eq!(resolve(None, None), Environment::Local);
}

#[test]
fn la_compilation_pose_l_environnement() {
    assert_eq!(resolve(None, Some("production")), Environment::Production);
    assert_eq!(resolve(None, Some("preprod")), Environment::Preproduction);
}

#[test]
fn le_lancement_prime_sur_la_compilation() {
    // Rejouer un binaire de production en local ne doit pas salir la
    // production.
    assert_eq!(
        resolve(Some("local"), Some("production")),
        Environment::Local
    );
}

#[test]
fn une_valeur_inconnue_ne_prend_pas_la_place_du_reste() {
    // Une faute de frappe au lancement ne doit pas effacer ce que la
    // compilation avait déclaré.
    assert_eq!(
        resolve(Some("prodction"), Some("production")),
        Environment::Production
    );
    assert_eq!(resolve(Some("n'importe quoi"), None), Environment::Local);
}

#[test]
fn les_alias_usuels_sont_acceptes() {
    for (texte, attendu) in [
        ("dev", Environment::Development),
        ("DEV", Environment::Development),
        ("staging", Environment::Preproduction),
        ("pre-prod", Environment::Preproduction),
        (" prod ", Environment::Production),
    ] {
        assert_eq!(Environment::parse(texte), Some(attendu), "pour « {texte} »");
    }
}

#[test]
fn seuls_les_environnements_publies_sont_dits_deployes() {
    assert!(!Environment::Local.is_deployed());
    assert!(!Environment::Development.is_deployed());
    assert!(Environment::Preproduction.is_deployed());
    assert!(Environment::Production.is_deployed());
}
