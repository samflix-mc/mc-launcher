use super::resolve;
use crate::environment::Environment;

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

/// `current` et `origin` lisent la même variable, et c'est leur accord qui
/// compte : un diagnostic annonçant « production » et « défaut, aucune
/// déclaration » sur la même exécution envoie chercher au mauvais endroit.
#[test]
fn le_diagnostic_dit_d_ou_vient_l_environnement() {
    use super::{COMPILED, current, origin};

    let garde = crate::essais::variables();
    garde.poser("SAMFLIX_ENV", "staging");
    assert_eq!(current(), Environment::Preproduction);
    assert_eq!(origin(), "variable SAMFLIX_ENV au lancement");

    // Une valeur illisible ne doit pas être annoncée comme une déclaration :
    // c'est précisément le cas où l'on cherche pourquoi l'environnement n'est
    // pas celui qu'on croyait.
    garde.poser("SAMFLIX_ENV", "prodction");
    assert_ne!(origin(), "variable SAMFLIX_ENV au lancement");

    // Sans déclaration au lancement, il ne reste que ce que la compilation a
    // pu figer : rien sur un poste, « development » sur la CI, qui compile
    // avec la variable posée. Ce test dit l'accord des deux réponses ; il ne
    // peut pas dire laquelle, sans quoi il mesurerait le runner.
    garde.retirer("SAMFLIX_ENV");
    match COMPILED.and_then(Environment::parse) {
        Some(compile) => {
            assert_eq!(current(), compile);
            assert_eq!(origin(), "SAMFLIX_ENV figé à la compilation");
        }
        None => {
            assert_eq!(current(), Environment::Local);
            assert_eq!(origin(), "défaut, aucune déclaration");
        }
    }
}

#[test]
fn chaque_environnement_a_le_nom_que_sentry_attend() {
    for (env, nom) in [
        (Environment::Local, "local"),
        (Environment::Development, "development"),
        (Environment::Preproduction, "preproduction"),
        (Environment::Production, "production"),
    ] {
        assert_eq!(env.as_str(), nom);
        assert_eq!(Environment::parse(nom), Some(env));
    }
}
