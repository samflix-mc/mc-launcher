use super::lines;
use crate::commands::fixtures::{Workshop, entry, lockfile};

/// An install result, reduced to what the report looks at.
fn result(workshop: &Workshop) -> mc_pack::Outcome {
    let options = workshop.options();
    mc_pack::Outcome {
        instance: options.layout.instance("samflix"),
        server_dir: workshop.root.join("server"),
        java: mc_java::Java {
            path: std::path::PathBuf::from("/usr/bin/java"),
            version: mc_java::parse_major("21.0.5").map_or_else(
                || panic!("readable version"),
                |major| mc_java::Version {
                    full: "21.0.5".into(),
                    major,
                },
            ),
            origin: mc_java::Origin::System,
        },
        neoforge: "21.1.250".into(),
        assets_downloaded: 0,
        libraries: 0,
        client_mods: 42,
        server_mods: 7,
        removed: Vec::new(),
        lock: lockfile(vec![entry("jei", "both")]),
        lock_path: workshop.root.join("samflix.lock.json"),
        previous_lock: None,
        source: "packs/samflix.json".into(),
        from_cache: false,
        drifts: Vec::new(),
        purge: Default::default(),
    }
}

/// The report says where the game was placed and how many mods on each
/// side: it's what a player rereads, and what they paste when asking for
/// help.
#[test]
fn the_report_says_where_and_how_many() {
    let workshop = Workshop::new("install-report");
    let rendered = lines(&result(&workshop)).join("\n");

    assert!(rendered.contains("samflix"), "{rendered}");
    assert!(rendered.contains("42 client-side"), "{rendered}");
    assert!(rendered.contains("7 server-side"), "{rendered}");
    assert!(rendered.contains("samflix.lock.json"), "{rendered}");
}

/// An offline install says so: the player doesn't have the published pack
/// but their local copy, and that's the first thing to know when an
/// expected version is missing.
#[test]
fn a_local_copy_is_announced_as_such() {
    let workshop = Workshop::new("install-cache");
    let mut outcome = result(&workshop);

    assert!(!lines(&outcome).join("\n").contains("offline"));

    outcome.from_cache = true;
    assert!(lines(&outcome).join("\n").contains("offline"));
}

/// Removed mods and changes only appear if there are any. A title followed
/// by nothing would send someone looking for a change that never happened;
/// staying silent would suggest an update did nothing.
#[test]
fn empty_lists_are_not_announced() {
    let workshop = Workshop::new("install-lists");
    let mut outcome = result(&workshop);

    let plain = lines(&outcome).join("\n");
    assert!(!plain.contains("removed"), "{plain}");
    assert!(!plain.contains("Changes"), "{plain}");

    outcome.removed = vec!["old-mod.jar".into()];
    outcome.previous_lock = Some(lockfile(Vec::new()));
    let verbose = lines(&outcome).join("\n");
    assert!(verbose.contains("old-mod.jar"), "{verbose}");
    assert!(verbose.contains("Changes"), "{verbose}");
    assert!(
        verbose.contains("jei"),
        "the added mod is missing: {verbose}"
    );
}
