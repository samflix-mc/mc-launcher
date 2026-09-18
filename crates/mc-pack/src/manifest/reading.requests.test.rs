use super::{Manifest, ModEntry};
use mc_mods::{Channel, Origin, Side};

use crate::manifest::fixtures::base;

#[test]
fn the_manifest_s_java_wins_over_mojang_s() {
    let mut m = base();
    assert_eq!(m.java_major(Some(21)), 21);
    m.java = Some(22);
    assert_eq!(m.java_major(Some(21)), 22);
}

/// Descriptors from before 1.17 have no `javaVersion` block. The pack's
/// manifest is then the only source, and 21 as a last resort — the only
/// place in the repo where this number is hardcoded.
#[test]
fn without_a_mojang_requirement_the_manifest_or_the_default() {
    let mut m = base();
    assert_eq!(m.java_major(None), 21);
    m.java = Some(17);
    assert_eq!(m.java_major(None), 17);
}

#[test]
fn the_side_is_read_from_the_text() {
    let entry = ModEntry {
        slug: "embeddium".into(),
        source: None,
        file: None,
        version: None,
        side: Some("client".into()),
        channel: None,
    };
    assert_eq!(entry.to_request().unwrap().side, Some(Side::Client));
}

#[test]
fn an_unknown_side_is_rejected() {
    let entry = ModEntry {
        slug: "x".into(),
        source: None,
        file: None,
        version: None,
        side: Some("both-sides".into()),
        channel: None,
    };
    assert!(entry.to_request().is_err());
}

#[test]
fn json_round_trip() {
    let json = r#"{
        "schema": 1,
        "name": "samflix",
        "minecraft": "1.21.1",
        "loader": { "type": "neoforge", "version": "latest" },
        "mods": [
            { "slug": "jei" },
            { "slug": "jade", "file": "eYz2YBGT", "source": "modrinth" },
            { "slug": "attributefix", "side": "both", "channel": "beta" }
        ]
    }"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    manifest.check().unwrap();
    assert!(manifest.loader.is_latest());
    assert_eq!(manifest.mods.len(), 3);
    assert_eq!(manifest.mods[1].source, Some(Origin::Modrinth));
    assert_eq!(manifest.mods[2].channel, Some(Channel::Beta));

    // What the manifest requests is what the resolver will go fetch: an
    // empty list would install a pack with no mods, without any error
    // reporting it — the game would start, bare.
    let requests = manifest.requests().expect("the manifest is consistent");
    assert_eq!(requests.len(), 3, "{requests:?}");
    assert_eq!(requests[1].source, Some(Origin::Modrinth));
    assert_eq!(requests[2].channel, Some(Channel::Beta));
}
