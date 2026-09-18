use super::super::Source;
use crate::fixtures::{MANIFEST, Workshop, entry, lock};

fn layout(workshop: &Workshop) -> mc_instance::Layout {
    mc_instance::Layout::new(workshop.root.clone())
}

fn client() -> mc_dl::Downloader {
    mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap()
}

/// An address is told apart from a path by its protocol; without this rule,
/// installing from the repo and installing from the site would need two
/// commands for the same gesture.
#[test]
fn an_address_is_told_apart_from_a_path_by_its_protocol() {
    let workshop = Workshop::new("source-parse");
    let layout = layout(&workshop);

    for address in [
        "https://mc-launcher.ggy.info/pack/samflix.json",
        "HTTP://example.invalid/p.json",
    ] {
        let source = Source::parse(address, &layout);
        assert!(source.is_remote(), "\"{address}\" should be an address");
        assert_eq!(source.describe(), address);
        assert!(source.local_path().is_none());
    }

    for path in ["packs/samflix.json", "./samflix.json", "/tmp/p.json"] {
        let source = Source::parse(path, &layout);
        assert!(!source.is_remote(), "\"{path}\" should be a path");
        assert_eq!(source.local_path().unwrap(), std::path::Path::new(path));
    }
}

#[tokio::test]
async fn a_local_pack_is_read_with_its_lock() {
    let workshop = Workshop::new("source-file");
    let manifest = workshop.write("samflix.json", MANIFEST.as_bytes());
    lock(vec![entry("jei", "both", None)])
        .save(&workshop.root.join("samflix.lock.json"))
        .unwrap();

    let pack = Source::parse(manifest.to_str().unwrap(), &layout(&workshop))
        .load(&client())
        .await
        .expect("the pack reads");

    assert_eq!(pack.manifest.name, "samflix");
    assert_eq!(pack.lock.as_ref().unwrap().mods.len(), 1);
    // A local pack is resolved: its builds are looked up, not replayed.
    assert!(!pack.replay);
    assert!(!pack.from_cache);
}

/// The first time a local pack is resolved, there's no lock — and that's not
/// an error.
#[tokio::test]
async fn a_local_pack_without_a_lock_still_reads() {
    let workshop = Workshop::new("source-no-lock");
    let manifest = workshop.write("samflix.json", MANIFEST.as_bytes());

    let pack = Source::parse(manifest.to_str().unwrap(), &layout(&workshop))
        .load(&client())
        .await
        .unwrap();

    assert!(pack.lock.is_none());
    assert!(pack.lock_path.ends_with("samflix.lock.json"));
}

/// `launch` and `verify` speak of the installation currently on disk: going
/// to fetch the published pack would make them describe a different pack,
/// and launching a session would stop working without a network.
#[test]
fn local_reading_does_not_touch_the_network() {
    let workshop = Workshop::new("source-local");
    let manifest = workshop.write("samflix.json", MANIFEST.as_bytes());

    let pack = Source::parse(manifest.to_str().unwrap(), &layout(&workshop))
        .load_local()
        .expect("the pack is there");

    assert_eq!(pack.manifest.name, "samflix");
    assert!(!pack.replay);
}

/// A remote pack that's never been installed can't be read locally: saying
/// so beats returning an empty manifest.
#[test]
fn a_never_installed_remote_pack_says_so() {
    let workshop = Workshop::new("source-never");
    let source = Source::parse("https://example.invalid/samflix.json", &layout(&workshop));

    let error = source.load_local().expect_err("nothing cached");
    assert!(
        format!("{error:#}").contains("mc-pack install"),
        "{error:#}"
    );
}

#[test]
fn a_missing_local_manifest_says_so_with_its_path() {
    let workshop = Workshop::new("source-missing");
    let source = Source::parse(
        workshop.root.join("nowhere.json").to_str().unwrap(),
        &layout(&workshop),
    );

    let error = source.load_local().expect_err("nothing at this spot");
    assert!(format!("{error:#}").contains("nowhere"), "{error:#}");
}
