use super::version;
use crate::fixtures::{Workshop, lock};
use crate::manifest::Manifest;

fn client() -> mc_dl::Downloader {
    mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap()
}

fn manifest(loader_version: &str) -> Manifest {
    let raw = format!(
        r#"{{"schema":1,"name":"samflix","minecraft":"1.21.1",
             "loader":{{"type":"neoforge","version":"{loader_version}"}}}}"#
    );
    Manifest::parse(raw.as_bytes()).unwrap()
}

/// The lock is authoritative when it's replayed: a player doesn't choose
/// their NeoForge version, or they'd show up on the server with a different
/// loader.
#[tokio::test]
async fn replay_takes_the_lock_version() {
    let workshop = Workshop::new("loader-replay");
    let path = workshop.root.join("samflix.lock.json");

    let placed = version(
        &manifest("latest"),
        Some(&lock(Vec::new())),
        &path,
        true,
        &client(),
    )
    .await
    .unwrap();

    // The manifest's "latest" is ignored: it's the lock that decides, and no
    // network call happens.
    assert_eq!(placed, "21.1.250");
}

/// A version pinned in the manifest is taken as is: that's the point of
/// pinning.
#[tokio::test]
async fn a_pinned_version_is_taken_as_is() {
    let workshop = Workshop::new("loader-pinned");
    let path = workshop.root.join("samflix.lock.json");

    let placed = version(&manifest("21.1.100"), None, &path, false, &client())
        .await
        .unwrap();

    assert_eq!(placed, "21.1.100");
}

/// Replaying without a lock makes no sense, and the message must name the
/// file that was expected.
#[tokio::test]
async fn a_replay_without_a_lock_names_the_expected_file() {
    let workshop = Workshop::new("loader-no-lock");
    let path = workshop.root.join("samflix.lock.json");

    let error = version(&manifest("latest"), None, &path, true, &client())
        .await
        .expect_err("nothing to replay");

    let text = format!("{error:#}");
    assert!(text.contains("samflix.lock.json"), "{text}");
    assert!(text.contains("nothing to replay"), "{text}");
}
