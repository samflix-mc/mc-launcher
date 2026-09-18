use super::{LockedLoader, LockedMod, Lockfile, Path};
use mc_mods::Origin;

#[test]
fn the_lockfile_sits_next_to_the_manifest() {
    assert_eq!(
        Lockfile::path_for(Path::new("packs/samflix.json")),
        Path::new("packs/samflix.lock.json")
    );
}

/// What deep verification will compare a jar against: the strongest of the
/// two, and the SHA-1 alone for lockfiles written before the field existed.
#[test]
fn the_lockfile_offers_the_strongest_digest_it_carries() {
    let mut m = locked("jade", "f", "1.0");
    assert_eq!(m.checksum(), None);

    m.sha1 = Some("aa".into());
    assert_eq!(m.checksum(), Some(mc_dl::Checksum::Sha1("aa".into())));

    m.sha512 = Some("bb".into());
    assert_eq!(m.checksum(), Some(mc_dl::Checksum::Sha512("bb".into())));
}

pub(crate) fn locked(slug: &str, file: &str, version: &str) -> LockedMod {
    LockedMod {
        slug: slug.into(),
        name: slug.into(),
        source: Origin::Modrinth,
        project: slug.into(),
        file: file.into(),
        version: version.into(),
        channel: mc_mods::Channel::Release,
        file_name: format!("{slug}.jar"),
        url: format!("https://example.invalid/{slug}.jar"),
        sha1: None,
        sha512: None,
        size: 0,
        side: "both".into(),
        reason: "requested by the manifest".into(),
        provides: vec![slug.into()],
    }
}

pub(crate) fn lock(mods: Vec<LockedMod>) -> Lockfile {
    Lockfile {
        schema: 1,
        name: "test".into(),
        version: None,
        generated: "2025-01-01T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        generation: 0,
        servers: Default::default(),
        mods,
        unresolved: Vec::new(),
    }
}
