use super::Candidate;
use crate::{Channel, Origin, Side};

fn candidate(sha1: Option<&str>, sha512: Option<&str>) -> Candidate {
    Candidate {
        origin: Origin::Modrinth,
        project_id: "p".into(),
        slug: "s".into(),
        name: "N".into(),
        version_id: "v".into(),
        version_number: "1.0".into(),
        display_name: "1.0".into(),
        channel: Channel::Release,
        file_name: "s.jar".into(),
        url: "https://example.invalid/s.jar".into(),
        sha1: sha1.map(str::to_string),
        sha512: sha512.map(str::to_string),
        size: 0,
        published: "2025-01-01".into(),
        project_side: Side::Both,
        declared_deps: Vec::new(),
        page_url: None,
        redistributable: true,
    }
}

/// What holds for most of a pack: Modrinth gives both, and the strong one is
/// what gets checked against the downloaded file.
#[test]
fn the_strong_digest_wins_over_the_sha1() {
    let c = candidate(Some("aa"), Some("bb"));
    assert_eq!(c.checksum(), Some(mc_dl::Checksum::Sha512("bb".into())));
}

/// CurseForge publishes nothing stronger: refusing the SHA-1 would amount to
/// installing its jars without checking them at all.
#[test]
fn absent_the_sha1_stays_usable() {
    let c = candidate(Some("aa"), None);
    assert_eq!(c.checksum(), Some(mc_dl::Checksum::Sha1("aa".into())));
    assert_eq!(candidate(None, None).checksum(), None);
}
