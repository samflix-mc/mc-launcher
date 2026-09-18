use super::super::Source;
use crate::fixtures::{MANIFEST, Workshop, entry, lock};
use crate::manifest::Manifest;

fn layout(workshop: &Workshop) -> mc_instance::Layout {
    mc_instance::Layout::new(workshop.root.clone())
}

/// An already-installed remote pack is read back from its copy, without a
/// network: exactly the moment you want to play.
#[test]
fn an_installed_remote_pack_is_read_back_from_its_copy() {
    let workshop = Workshop::new("local-cache");
    let layout = layout(&workshop);
    let url = "https://mc-launcher.ggy.info/pack/samflix.json";
    let source = Source::parse(url, &layout);

    let Source::Remote { cache_dir, .. } = &source else {
        panic!("an address gives a remote source");
    };
    std::fs::create_dir_all(cache_dir).unwrap();
    Manifest::parse(MANIFEST.as_bytes())
        .unwrap()
        .save(&cache_dir.join("samflix.json"))
        .unwrap();
    lock(vec![entry("jei", "both", None)])
        .save(&cache_dir.join("samflix.lock.json"))
        .unwrap();

    let pack = source.load_local().expect("the copy is there");

    assert_eq!(pack.manifest.name, "samflix");
    // The remote lock is authoritative: its builds are replayed, not looked up.
    assert!(pack.replay);
    assert_eq!(pack.lock.unwrap().mods.len(), 1);
}

/// The cache is organized by host: flat, switching from dev to production
/// would silently overwrite the first, and a network outage would surface
/// the pack from the wrong environment.
#[test]
fn the_copies_of_two_environments_do_not_step_on_each_other() {
    let workshop = Workshop::new("local-hosts");
    let layout = layout(&workshop);

    let production = Source::parse("https://mc-launcher.ggy.info/pack/samflix.json", &layout);
    let dev = Source::parse(
        "https://mc-launcher-dev.ggy.info/pack/samflix.json",
        &layout,
    );

    let (Source::Remote { cache_dir: a, .. }, Source::Remote { cache_dir: b, .. }) =
        (&production, &dev)
    else {
        panic!("two remote sources");
    };
    assert_ne!(a, b, "the two environments share the same cache");
}

/// `lock` needs a file to rewrite: resolving a remote pack would have
/// nowhere to place its result.
#[test]
fn a_remote_pack_has_no_path_to_edit() {
    let workshop = Workshop::new("local-path");
    let layout = layout(&workshop);

    assert!(
        Source::parse("https://example.invalid/p.json", &layout)
            .local_path()
            .is_none()
    );
    assert!(
        Source::parse("packs/samflix.json", &layout)
            .local_path()
            .is_some()
    );
}
