use super::version_entry;
use crate::vanilla::descriptor::ManifestVersion;

fn version(id: &str) -> ManifestVersion {
    ManifestVersion {
        id: id.to_string(),
        url: format!("https://example.invalid/{id}.json"),
        sha1: "abc123".into(),
    }
}

/// Mojang's manifest lists several hundred versions, snapshots included.
/// Picking the wrong entry would install a different game than the one
/// requested, with its own libraries and assets — and the pack wouldn't
/// start, for a reason that wouldn't show up anywhere.
#[test]
fn only_the_requested_version_is_kept() {
    let published = vec![version("1.21.4"), version("1.21.1"), version("25w07a")];

    let found = version_entry(published, "1.21.1").expect("the version is published");
    assert_eq!(found.id, "1.21.1");
    assert!(found.url.contains("1.21.1"));
}

/// A version Mojang doesn't publish yields nothing — this is what happens
/// with a typo in the pack manifest, and the caller turns it into a message
/// that names it.
#[test]
fn a_version_missing_from_the_manifest_yields_nothing() {
    assert!(version_entry(vec![version("1.21.1")], "1.21.9").is_none());
    assert!(version_entry(Vec::new(), "1.21.1").is_none());
}
