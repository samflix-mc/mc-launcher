use super::*;
use crate::manifest::essais::avec_serveurs;



#[test]
fn le_serveur_suit_l_environnement() {
    let manifest = avec_serveurs();
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
        Some("78.46.100.5:25566".into())
    );
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info:25565".into()),
        "un port déclaré s'écrit, fût-il le port par défaut"
    );
}


#[test]
fn un_binaire_local_rejoint_la_dev() {
    // Un binaire compilé à la main est un binaire de travail. Le faire
    // tomber sur la production reviendrait à envoyer quelqu'un qui essaie
    // là où d'autres jouent.
    let manifest = avec_serveurs();
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Local)
            .map(Server::address),
        manifest
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
    );
}

#[test]
fn une_preproduction_sans_serveur_ne_lance_rien() {
    // Elle n'a pas de serveurs Minecraft derrière elle, et ce n'est pas un
    // oubli : le nœud est unique, chaque réseau complet coûte de la RAM.
    assert!(
        avec_serveurs()
            .server_for(mc_log::Environment::Preproduction)
            .is_none()
    );
}




#[test]
fn les_cles_canoniques_ne_posent_aucun_probleme() {
    assert!(avec_serveurs().problemes_de_serveurs().is_empty());
}


#[test]
fn un_binaire_local_lit_la_cle_development() {
    assert_eq!(
        Manifest::environnement_serveur(mc_log::Environment::Local),
        mc_log::Environment::Development
    );
    assert_eq!(
        Manifest::environnement_serveur(mc_log::Environment::Production),
        mc_log::Environment::Production
    );
}

#[test]
fn un_manifeste_sans_serveurs_reste_lisible() {
    // Le champ est arrivé après les premiers packs : les manifestes qui
    // l'ignorent doivent continuer de se lire tels quels.
    let brut = br#"{"schema":1,"name":"essai","minecraft":"1.21.1",
                    "loader":{"type":"neoforge","version":"latest"}}"#;
    let manifest = Manifest::parse(brut).expect("manifeste sans serveurs");
    assert!(manifest.servers.is_empty());
    assert!(
        manifest
            .server_for(mc_log::Environment::Production)
            .is_none()
    );
}
