use super::{PathBuf, cache_dir_for, file_name_of, is_url, lock_url_for};
use crate::source::Source;

#[test]
fn an_address_is_recognized_by_its_protocol() {
    assert!(is_url("https://mc-launcher.ggy.info/pack/samflix.json"));
    assert!(is_url("HTTP://example.invalid/x.json"));
    assert!(!is_url("packs/samflix.json"));
    assert!(!is_url("/var/tmp/samflix.json"));
}

#[test]
fn the_remote_lock_is_derived_from_the_manifest() {
    assert_eq!(
        lock_url_for("https://example.invalid/pack/samflix.json"),
        "https://example.invalid/pack/samflix.lock.json"
    );
}

#[test]
fn the_file_name_ignores_the_query_string() {
    assert_eq!(
        file_name_of("https://example.invalid/pack/samflix.json?v=3"),
        "samflix.json"
    );
    assert_eq!(file_name_of("https://example.invalid/pack/"), "pack.json");
}

#[test]
fn two_environments_do_not_share_their_cache() {
    let layout = mc_instance::Layout::new(PathBuf::from("/data"));
    let prod = cache_dir_for("https://mc-launcher.ggy.info/pack/samflix.json", &layout);
    let dev = cache_dir_for(
        "https://mc-launcher-dev.ggy.info/pack/samflix.json",
        &layout,
    );
    assert_ne!(prod, dev);
    assert!(prod.ends_with("mc-launcher.ggy.info"));
}

#[test]
fn a_path_stays_a_path() {
    let layout = mc_instance::Layout::new(PathBuf::from("/data"));
    let source = Source::parse("packs/samflix.json", &layout);
    assert!(!source.is_remote());
    assert_eq!(source.describe(), "packs/samflix.json");
}
