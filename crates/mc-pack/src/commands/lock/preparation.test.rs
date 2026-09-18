use super::manifest_to_lock;
use crate::commands::fixtures::{MANIFEST, Workshop};
use mc_pack::source::Source;

/// Resolving produces a lock, and a lock has to be placed somewhere. A
/// published pack doesn't offer that place — and doesn't need it: it
/// already arrives locked, which is exactly what makes it a published pack.
#[test]
fn a_published_pack_does_not_need_to_be_locked() {
    let workshop = Workshop::new("lock-remote");
    let source = Source::parse(
        "https://mc-launcher.ggy.info/pack/samflix.json",
        &workshop.options().layout,
    );

    let error = manifest_to_lock(&source).expect_err("nothing to edit");
    let text = format!("{error:#}");
    assert!(text.contains("give it a path"), "{text}");
    assert!(text.contains("mc-content"), "{text}");
}

#[test]
fn a_manifest_from_the_repo_is_accepted() {
    let workshop = Workshop::new("lock-local");
    let path = workshop.root.join("samflix.json");
    std::fs::write(&path, MANIFEST).unwrap();
    let source = Source::parse(path.to_str().unwrap(), &workshop.options().layout);

    let (returned, manifest) = manifest_to_lock(&source).expect("the manifest is there");
    assert_eq!(returned, path);
    assert_eq!(manifest.name, "samflix");
}

/// This is the one and only place an unreadable "servers" key gets
/// rejected: reading the manifest also applies to the downloaded pack, and
/// a binary that refused an unknown environment would stop the day
/// mc-content declares one more.
#[test]
fn a_server_key_that_would_never_be_read_is_rejected() {
    let workshop = Workshop::new("lock-key");
    let path = workshop.root.join("samflix.json");
    std::fs::write(
        &path,
        r#"{"schema":1,"name":"samflix","minecraft":"1.21.1",
            "loader":{"type":"neoforge","version":"21.1.250"},
            "servers":{"prodction":{"host":"mc.ggy.info"}}}"#,
    )
    .unwrap();
    let source = Source::parse(path.to_str().unwrap(), &workshop.options().layout);

    let error = manifest_to_lock(&source).expect_err("unreadable key");
    let text = format!("{error:#}");
    assert!(text.contains("prodction"), "{text}");
    assert!(text.contains("never be read"), "{text}");
}

#[test]
fn a_missing_manifest_is_reported_with_its_path() {
    let workshop = Workshop::new("lock-missing");
    let path = workshop.root.join("nowhere.json");
    let source = Source::parse(path.to_str().unwrap(), &workshop.options().layout);

    let error = manifest_to_lock(&source).expect_err("nothing at this location");
    assert!(format!("{error:#}").contains("nowhere"), "{error:#}");
}
