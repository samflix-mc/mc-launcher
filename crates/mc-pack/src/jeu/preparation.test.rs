use super::{Confort, options_de_lancement};
use mc_instance::launch::QuickPlay;

/// Deux réglages, et deux façons de les perdre en silence. Sans mémoire
/// demandée, la JVM retombe sur son défaut — un quart de la machine, ce qui ne
/// suffit pas à un modpack et donne un `OutOfMemoryError` au bout de vingt
/// minutes de partie.
#[test]
fn la_memoire_demandee_est_transmise_au_jeu() {
    let options = options_de_lancement(
        &Confort {
            memoire_mo: Some(6144),
            ..Default::default()
        },
        None,
    );
    assert_eq!(options.memory_mb, Some(6144));

    // À défaut, on laisse la JVM décider : c'est le cas d'un lancement sans
    // « --memoire ».
    assert_eq!(
        options_de_lancement(&Confort::default(), None).memory_mb,
        None
    );
}

/// Sans Quick Play, le jeu s'ouvre sur son menu au lieu de rejoindre le
/// serveur — et l'on croit que le pack n'en déclare pas.
#[test]
fn le_serveur_a_rejoindre_devient_un_quick_play() {
    let options = options_de_lancement(&Confort::default(), Some("mc.exemple.fr".to_string()));

    match options.quick_play {
        Some(QuickPlay::Multiplayer(hote)) => assert_eq!(hote, "mc.exemple.fr"),
        autre => panic!("le serveur n'est pas rejoint : {autre:?}"),
    }

    // Sans cible — la préproduction n'a pas de serveur —, le menu est le bon
    // comportement.
    assert!(
        options_de_lancement(&Confort::default(), None)
            .quick_play
            .is_none()
    );
}

/// **La résolution et le plein écran arrivent jusqu'au jeu.**
///
/// Les deux champs pouvaient être SUPPRIMÉS de la structure sans qu'un test
/// bronche : c'est ce que la mutation a montré. Le symptôme aurait été le pire
/// possible — le joueur règle la taille de sa fenêtre dans Configuration, la
/// valeur est écrite dans `reglages.json`, relue, validée, transmise… et
/// ignorée au dernier maillon. Rien dans le journal, rien à l'écran, et un
/// réglage qui « ne marche pas » sans qu'on sache où il se perd.
///
/// `resolution` fait plus que dimensionner la fenêtre : elle active
/// `has_custom_resolution` dans le descripteur de Mojang, ce qui débloque des
/// arguments conditionnels. La perdre change donc la ligne de commande, et pas
/// seulement la taille.
#[test]
fn la_resolution_et_le_plein_ecran_arrivent_jusqu_au_jeu() {
    let options = options_de_lancement(
        &Confort {
            resolution: Some((1600, 900)),
            plein_ecran: true,
            ..Default::default()
        },
        None,
    );

    assert_eq!(options.resolution, Some((1600, 900)));
    assert!(options.plein_ecran);
}

/// Et les défauts sont bien des défauts : rien d'imposé au jeu.
///
/// Le pendant du test précédent. Poser une résolution que personne n'a
/// demandée activerait `has_custom_resolution` sur tous les postes, donc des
/// arguments que Mojang réserve à ce cas.
#[test]
fn sans_reglage_rien_n_est_impose_au_jeu() {
    let options = options_de_lancement(&Confort::default(), None);

    assert_eq!(options.resolution, None);
    assert!(!options.plein_ecran);
}
