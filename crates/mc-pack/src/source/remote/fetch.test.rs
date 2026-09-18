use super::{copy_complete, load_remote};
use crate::fixtures::{MANIFEST, Workshop, entry, lock};
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

fn client() -> mc_dl::Downloader {
    mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap()
}

fn lock_json() -> String {
    serde_json::to_string(&lock(vec![entry("jei", "both", None)])).unwrap()
}

/// The manifest and the lock are fetched together, and the lock is replayed
/// as-is: a player resolves nothing, otherwise their machine would pick its
/// own versions and they'd show up on the server with registries that no
/// longer match.
#[tokio::test]
async fn a_remote_pack_is_downloaded_with_its_lock_and_cached() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("remote-nominal");
    server.json("/pack/samflix.json", MANIFEST);
    server.json("/pack/samflix.lock.json", &lock_json());

    let cache = workshop.root.join("cache");
    let pack = load_remote(&server.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect("the pack is fetched");

    assert_eq!(pack.manifest.name, "samflix");
    assert!(pack.replay, "the remote lock is authoritative");
    assert!(!pack.from_cache);
    // The copy is left behind for offline launches.
    assert!(cache.join("samflix.json").is_file());
    assert!(cache.join("samflix.lock.json").is_file());
}

/// Playing with yesterday's pack beats not playing.
#[tokio::test]
async fn offline_the_last_known_copy_takes_over() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("remote-cache");
    let cache = workshop.root.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    Manifest::parse(MANIFEST.as_bytes())
        .unwrap()
        .save(&cache.join("samflix.json"))
        .unwrap();
    lock(vec![entry("jei", "both", None)])
        .save(&cache.join("samflix.lock.json"))
        .unwrap();

    // The server serves nothing: this is the network outage.
    server.code("/pack/samflix.json", 500);

    let pack = load_remote(&server.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect("the local copy saves the launch");

    assert!(pack.from_cache, "the fallback isn't reported");
    assert!(pack.replay);
    assert_eq!(pack.manifest.name, "samflix");
}

/// Nothing is cached before it's been read back: an HTML error page served
/// as HTTP 200 would otherwise replace a valid pack with nothing.
#[tokio::test]
async fn an_unreadable_response_does_not_replace_the_valid_copy() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("remote-html");
    let cache = workshop.root.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    Manifest::parse(MANIFEST.as_bytes())
        .unwrap()
        .save(&cache.join("samflix.json"))
        .unwrap();
    lock(vec![entry("jei", "both", None)])
        .save(&cache.join("samflix.lock.json"))
        .unwrap();

    server.json("/pack/samflix.json", "<html><body>503</body></html>");
    server.json("/pack/samflix.lock.json", &lock_json());

    let pack = load_remote(&server.url("/pack/samflix.json"), &cache, &client())
        .await
        .unwrap();

    assert!(pack.from_cache);
    // The valid copy is untouched: it wasn't overwritten by the HTML.
    let reread = Lockfile::load(&cache.join("samflix.lock.json")).unwrap();
    assert_eq!(reread.mods.len(), 1);
}

/// A fresh manifest paired with the previous lock would describe a pack
/// nobody has ever published: both or neither.
#[tokio::test]
async fn an_unreadable_lock_also_cancels_the_manifest() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("remote-broken-lock");
    let cache = workshop.root.join("cache");
    server.json("/pack/samflix.json", MANIFEST);
    server.json("/pack/samflix.lock.json", "not a lock");

    let error = load_remote(&server.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect_err("neither a usable pack, nor a copy");

    assert!(format!("{error:#}").contains("no copy"), "{error:#}");
    assert!(
        !cache.join("samflix.json").exists(),
        "an orphaned manifest was cached"
    );
}

#[tokio::test]
async fn without_network_or_copy_the_failure_names_the_cache() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("remote-nothing");
    let cache = workshop.root.join("cache");
    server.code("/pack/samflix.json", 404);

    let error = load_remote(&server.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect_err("nothing anywhere");

    let text = format!("{error:#}");
    assert!(text.contains("unusable"), "{text}");
    assert!(text.contains("cache"), "{text}");
}

/// A local copy is only usable when it's whole. A manifest without its lock
/// would resolve again right when the network is exactly what's
/// unreachable; a lock without its manifest doesn't say which pack it locks.
/// Requiring only one would let the installation carry on with half a pack.
#[test]
fn a_local_copy_is_only_usable_when_complete() {
    let workshop = Workshop::new("local-copy");
    let manifest = workshop.root.join("samflix.json");
    let local_lock = workshop.root.join("samflix.lock.json");

    assert!(!copy_complete(&manifest, &local_lock), "nothing on disk");

    std::fs::write(&manifest, MANIFEST).unwrap();
    assert!(
        !copy_complete(&manifest, &local_lock),
        "the lock is missing"
    );

    lock(vec![entry("jei", "both", None)])
        .save(&local_lock)
        .unwrap();
    assert!(copy_complete(&manifest, &local_lock));

    std::fs::remove_file(&manifest).unwrap();
    assert!(
        !copy_complete(&manifest, &local_lock),
        "the manifest is missing"
    );
}
