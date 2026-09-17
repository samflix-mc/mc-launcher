use super::options_de_lancement;
use mc_instance::launch::QuickPlay;

/// Deux réglages, et deux façons de les perdre en silence. Sans mémoire
/// demandée, la JVM retombe sur son défaut — un quart de la machine, ce qui ne
/// suffit pas à un modpack et donne un `OutOfMemoryError` au bout de vingt
/// minutes de partie.
#[test]
fn la_memoire_demandee_est_transmise_au_jeu() {
    let options = options_de_lancement(Some(6144), None);
    assert_eq!(options.memory_mb, Some(6144));

    // À défaut, on laisse la JVM décider : c'est le cas d'un lancement sans
    // « --memoire ».
    assert_eq!(options_de_lancement(None, None).memory_mb, None);
}

/// Sans Quick Play, le jeu s'ouvre sur son menu au lieu de rejoindre le
/// serveur — et l'on croit que le pack n'en déclare pas.
#[test]
fn le_serveur_a_rejoindre_devient_un_quick_play() {
    let options = options_de_lancement(None, Some("mc.exemple.fr".to_string()));

    match options.quick_play {
        Some(QuickPlay::Multiplayer(hote)) => assert_eq!(hote, "mc.exemple.fr"),
        autre => panic!("le serveur n'est pas rejoint : {autre:?}"),
    }

    // Sans cible — la préproduction n'a pas de serveur —, le menu est le bon
    // comportement.
    assert!(options_de_lancement(None, None).quick_play.is_none());
}
