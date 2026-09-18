use crate::manifest::ModEntry;
use crate::manifest::fixtures::base;

#[test]
fn a_name_that_escapes_the_directory_is_rejected() {
    // The manifest comes from the network since the remote pack became the
    // default source, and `deploy` deletes the .jar files from the
    // directory this name designates.
    for bad in [
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
        manifest.name = bad.into();
        assert!(
            manifest.check().is_err(),
            "“{bad}” should have been rejected"
        );
    }
}

#[test]
fn an_unusual_but_harmless_name_passes() {
    // This check doesn't judge good taste. Rejecting here what's merely
    // unexpected would doom a future pack across every launcher already
    // distributed — and a launcher that rejects the pack can no longer be
    // repaired.
    for good in ["samflix", "samflix v2", "pack.été-2026", "SAMFLIX_2"] {
        let mut manifest = base();
        manifest.name = good.into();
        assert!(
            manifest.check().is_ok(),
            "“{good}” should have been accepted"
        );
    }
}

#[test]
fn an_unknown_format_is_rejected() {
    let mut m = base();
    m.schema = 99;
    assert!(m.check().is_err());
}

#[test]
fn a_duplicate_mod_is_rejected() {
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
fn pinning_the_same_thing_twice_is_rejected() {
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

/// The nominal case: nothing exotic, and the check lets it through.
#[test]
fn a_minimal_manifest_is_accepted() {
    assert!(base().check().is_ok());
}
