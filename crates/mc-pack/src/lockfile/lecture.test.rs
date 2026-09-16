use super::*;
use mc_mods::Origin;

#[test]
fn le_verrou_se_place_a_cote_du_manifeste() {
    assert_eq!(
        Lockfile::path_for(Path::new("packs/samflix.json")),
        Path::new("packs/samflix.lock.json")
    );
}

/// Ce que la vérification profonde opposera au jar : la plus forte des
/// deux, et le SHA-1 seul pour les verrous écrits avant le champ.
#[test]
fn le_verrou_oppose_la_plus_forte_empreinte_qu_il_porte() {
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
        origin: Origin::Modrinth,
        project: slug.into(),
        file: file.into(),
        version: version.into(),
        file_name: format!("{slug}.jar"),
        url: format!("https://exemple.invalid/{slug}.jar"),
        sha1: None,
        sha512: None,
        size: 0,
        side: "both".into(),
        reason: "demandé par le manifeste".into(),
        provides: vec![slug.into()],
    }
}

pub(crate) fn lock(mods: Vec<LockedMod>) -> Lockfile {
    Lockfile {
        schema: 1,
        pack: "essai".into(),
        generated: "2025-01-01T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        mods,
        unresolved: Vec::new(),
    }
}
