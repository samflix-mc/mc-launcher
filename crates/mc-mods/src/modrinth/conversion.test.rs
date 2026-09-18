use super::{Project, Side, side_of};
use crate::modrinth::api::ApiFile;

fn project(client: &str, server: &str) -> Project {
    Project {
        id: "id".into(),
        slug: "slug".into(),
        title: "Title".into(),
        client_side: client.into(),
        server_side: server.into(),
    }
}

/// Modrinth publishes both digests; for a long time only the weaker one was
/// read, and that's the one that ended up in the lockfile.
#[test]
fn the_sha512_published_by_modrinth_is_kept() {
    let raw = r#"{
        "url": "https://cdn.modrinth.com/jade.jar",
        "filename": "jade.jar",
        "primary": true,
        "size": 1024,
        "hashes": {
            "sha1": "0a385a583a1e9413ecf2a47d00000000deadbeef",
            "sha512": "b6c782de87e7259d997e199200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
        }
    }"#;
    let file: ApiFile = serde_json::from_str(raw).expect("readable Modrinth file");
    assert_eq!(file.hashes.sha512.as_deref().map(str::len), Some(128));
    assert!(file.hashes.sha1.is_some());
}

#[test]
fn side_deduced_from_the_projects_metadata() {
    assert_eq!(side_of(&project("required", "unsupported")), Side::Client);
    assert_eq!(side_of(&project("unsupported", "required")), Side::Server);
    assert_eq!(side_of(&project("required", "required")), Side::Both);
    // JEI and Jade are "optional / optional": installed on both sides,
    // otherwise NeoForge registries would drift apart.
    assert_eq!(side_of(&project("optional", "optional")), Side::Both);
}
