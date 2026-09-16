//! Les clés que `problemes_de_serveurs` refuse, et pourquoi.

use crate::manifest::essais::avec_cle;

#[test]
fn une_cle_de_serveur_fautive_est_signalee() {
    // Sans ce contrôle, une faute de frappe ne provoque rien : la clé ne
    // correspond à aucun environnement, le jeu s'ouvre sur le menu, et
    // cela ressemble exactement à un pack qui n'aurait rien déclaré.
    let problemes = avec_cle("prodution", "mc.ggy.info").problemes_de_serveurs();
    assert_eq!(problemes.len(), 1);
    assert!(problemes[0].contains("prodution"));
}

#[test]
fn un_alias_est_signale_parce_qu_il_ne_serait_pas_lu() {
    // Environment::parse accepte « dev », mais server_for cherche
    // « development » : l'entrée passerait ici et resterait introuvable.
    let problemes = avec_cle("dev", "mc-dev.ggy.info").problemes_de_serveurs();
    assert_eq!(problemes.len(), 1);
    assert!(problemes[0].contains("development"));
}

#[test]
fn une_cle_local_est_signalee() {
    let problemes = avec_cle("local", "mc-dev.ggy.info").problemes_de_serveurs();
    assert_eq!(problemes.len(), 1);
    assert!(problemes[0].contains("development"));
}

#[test]
fn un_hote_vide_est_signale() {
    assert_eq!(
        avec_cle("production", "   ").problemes_de_serveurs().len(),
        1
    );
}
