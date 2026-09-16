use super::Server;
use crate::manifest::essais::{base, serveur};

#[test]
fn une_ipv6_est_mise_entre_crochets() {
    // Guava, dont Minecraft se sert pour lire l'adresse, refuse tout ce qui
    // porte plus d'un « : » sans crochets. Une IPv6 nue ne donnait donc pas
    // une mauvaise adresse : elle n'en donnait aucune, et le jeu s'ouvrait
    // sur le menu comme si le pack n'avait rien déclaré.
    assert_eq!(
        serveur("2001:db8::1", Some(25566)).address(),
        "[2001:db8::1]:25566"
    );
    assert_eq!(serveur("2001:db8::1", None).address(), "[2001:db8::1]");
    assert_eq!(
        serveur("[2001:db8::1]", Some(25566)).address(),
        "[2001:db8::1]:25566",
        "des crochets déjà posés ne se doublent pas"
    );
}

#[test]
fn un_nom_et_une_ipv4_restent_intacts() {
    assert_eq!(serveur("mc.ggy.info", None).address(), "mc.ggy.info");
    assert_eq!(
        serveur("78.46.100.5", Some(25566)).address(),
        "78.46.100.5:25566"
    );
}

#[test]
fn un_hote_deja_suffixe_d_un_port_est_signale() {
    // « mc.ggy.info:25566 » plus un champ « port » composerait
    // « mc.ggy.info:25566:25570 », que Minecraft refuse — et le refus est
    // muet, comme pour une IPv6 nue.
    let mut manifest = base();
    manifest.servers.insert(
        "production".into(),
        serveur("mc.ggy.info:25566", Some(25570)),
    );
    assert_eq!(manifest.problemes_de_serveurs().len(), 1);

    // Seul le doublon gêne : un hôte suffixé sans champ « port » compose
    // une adresse valable, et rien ne justifie de la refuser.
    let mut manifest = base();
    manifest
        .servers
        .insert("production".into(), serveur("mc.ggy.info:25566", None));
    assert!(manifest.problemes_de_serveurs().is_empty());
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info:25566".into())
    );
}
