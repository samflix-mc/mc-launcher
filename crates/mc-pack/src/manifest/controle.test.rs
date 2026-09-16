use crate::manifest::*;

use crate::manifest::essais::{base};


#[test]
fn un_nom_qui_sort_du_repertoire_est_refuse() {
    // Le manifeste vient du réseau depuis que le pack distant est la
    // source par défaut, et « deploy » supprime les .jar du répertoire que
    // ce nom désigne.
    for fautif in [
        "../../../../home/sam/Documents",
        "/home/sam/.minecraft",
        "..",
        ".",
        "",
        "   ",
        "samflix/../..",
        r"..\..\Windows",
    ] {
        let mut manifest = base();
        manifest.name = fautif.into();
        assert!(
            manifest.check().is_err(),
            "« {fautif} » aurait dû être refusé"
        );
    }
}

#[test]
fn un_nom_inhabituel_mais_sans_danger_passe() {
    // Ce contrôle ne juge pas du bon goût. Refuser ici ce qui est
    // seulement inattendu condamnerait un pack futur chez tous les
    // launchers déjà distribués — et un launcher qui refuse le pack ne
    // peut plus se dépanner.
    for correct in ["samflix", "samflix v2", "pack.été-2026", "SAMFLIX_2"] {
        let mut manifest = base();
        manifest.name = correct.into();
        assert!(
            manifest.check().is_ok(),
            "« {correct} » aurait dû être accepté"
        );
    }
}


#[test]
fn un_format_inconnu_est_refuse() {
    let mut m = base();
    m.schema = 99;
    assert!(m.check().is_err());
}

#[test]
fn un_mod_en_double_est_refuse() {
    let mut m = base();
    m.mods = vec![
        ModEntry {
            slug: "jei".into(),
            source: None,
            file: None,
            version: None,
            side: None,
            channel: None,
        },
        ModEntry {
            slug: "JEI".into(),
            source: None,
            file: None,
            version: None,
            side: None,
            channel: None,
        },
    ];
    assert!(m.check().is_err());
}

#[test]
fn epingler_deux_fois_la_meme_chose_est_refuse() {
    let mut m = base();
    m.mods = vec![ModEntry {
        slug: "jei".into(),
        source: None,
        file: Some("abcd1234".into()),
        version: Some("19.51.0.418".into()),
        side: None,
        channel: None,
    }];
    assert!(m.check().is_err());
}

/// Le cas nominal : rien d'exotique, et le contrôle laisse passer.
#[test]
fn un_manifeste_minimal_est_accepte() {
    assert!(base().check().is_ok());
}
