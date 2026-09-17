use super::doit_rejouer;

/// Deux raisons de rejouer un verrou, indépendantes l'une de l'autre : le pack
/// vient du réseau — c'est son verrou publié qui décide, pas la machine du
/// joueur — ou « --locked » l'exige sur un manifeste local. Les confondre
/// ferait résoudre à nouveau un pack publié, et le joueur n'aurait pas les
/// versions que le réseau a validées.
#[test]
fn un_pack_distant_ou_un_verrou_exige_se_rejouent() {
    assert!(doit_rejouer(true, false), "un pack distant se rejoue");
    assert!(doit_rejouer(false, true), "« --locked » l'exige");
    assert!(doit_rejouer(true, true));

    // Un manifeste local qu'on édite se résout à nouveau : c'est tout
    // l'intérêt de travailler dessus.
    assert!(!doit_rejouer(false, false));
}
