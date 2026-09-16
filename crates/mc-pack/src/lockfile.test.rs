use super::*;

#[test]
fn horodatage_iso8601() {
    assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    // Année bissextile, 29 février.
    assert_eq!(format_iso8601(1_709_164_800), "2024-02-29T00:00:00Z");
}

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

fn locked(slug: &str, file: &str, version: &str) -> LockedMod {
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

fn lock(mods: Vec<LockedMod>) -> Lockfile {
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

#[test]
fn le_diff_dit_ce_qui_a_bouge() {
    let avant = lock(vec![
        locked("jei", "a", "19.51"),
        locked("jade", "b", "15.10"),
    ]);
    let apres = lock(vec![
        locked("jei", "c", "19.56"),
        locked("bookshelf-lib", "d", "21.1.81"),
    ]);

    let lignes = apres.diff(&avant);
    assert!(lignes.contains(&"~ jei 19.51 → 19.56".to_string()));
    assert!(lignes.contains(&"+ bookshelf-lib 21.1.81".to_string()));
    assert!(lignes.contains(&"- jade 15.10".to_string()));
}

#[test]
fn rejouer_un_verrou_epingle_chaque_build() {
    let verrou = lock(vec![locked("jei", "9myHusbW", "19.56")]);
    let requests = verrou.requests();
    assert_eq!(requests[0].file.as_deref(), Some("9myHusbW"));
    assert_eq!(requests[0].source, Some(Origin::Modrinth));
    assert_eq!(requests[0].side, Some(Side::Both));
}
