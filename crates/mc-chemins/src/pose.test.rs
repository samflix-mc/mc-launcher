use super::{courants, poses};

/// Sans pose, `courants()` répond quand même — et répond ce que
/// l'environnement dit.
///
/// Les crates sont utilisables hors de l'application : `mc-pack` en ligne de
/// commande n'appelle jamais `poser`, et doit pourtant trouver ses données.
///
/// Ce test vit dans le même processus que les autres du crate, où AUCUNE pose
/// n'a lieu — la pose, elle, s'éprouve dans `tests/pose_unique.rs`, un binaire
/// à part. Deux tests d'un même binaire qui poseraient chacun le leur se
/// contrediraient selon leur ordre d'exécution.
#[test]
fn sans_pose_les_emplacements_viennent_de_l_environnement() {
    assert!(!poses(), "aucune pose ne doit avoir lieu dans ce binaire");
    assert_eq!(courants(), crate::du_systeme());
}

/// Et le repli ne mémorise pas : deux appels successifs relisent
/// l'environnement.
///
/// Sans cette propriété, le premier appelant figerait l'arborescence pour tout
/// le processus — et sept suites existantes, qui déplacent `XDG_DATA_HOME` en
/// cours de route, verraient ou non leur déplacement selon leur rang dans la
/// suite. Un test qui passe ou échoue selon son rang est pire qu'un test
/// absent.
#[test]
fn le_repli_ne_memorise_pas() {
    let avant = courants();

    // SAFETY : la variable est restaurée avant la fin du test, et ce crate n'a
    // aucun autre test qui la lise.
    let ancien = std::env::var_os("XDG_DATA_HOME");
    unsafe { std::env::set_var("XDG_DATA_HOME", "/ailleurs/pour-le-repli") };
    let pendant = courants();
    unsafe {
        match ancien {
            Some(valeur) => std::env::set_var("XDG_DATA_HOME", valeur),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }
    let apres = courants();

    // Sous macOS et Windows, XDG n'a aucun effet : le test ne porte alors que
    // sur le fait qu'on n'a rien figé, ce qu'attestent les deux autres égalités.
    if cfg!(all(unix, not(target_os = "macos"))) {
        assert_ne!(
            avant, pendant,
            "le repli a figé l'environnement du premier appelant"
        );
        assert!(pendant.donnees.starts_with("/ailleurs/pour-le-repli"));
    }
    assert_eq!(avant, apres, "l'environnement n'a pas été restauré");
}
