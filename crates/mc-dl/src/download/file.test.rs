use super::write_atomic;
use crate::{Check, Checksum, Downloader, Fetched};

fn folder(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mc-dl-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&path).ok();
    path
}

fn client() -> Downloader {
    Downloader::new(crate::USER_AGENT).unwrap()
}

fn sha1(bytes: &[u8]) -> Checksum {
    let sum = Checksum::Sha1(String::new());
    Checksum::Sha1(sum.of(bytes))
}

#[test]
fn atomic_write_leaves_no_leftover() {
    let dir = folder("atomic");
    let dest = dir.join("sub/folder/file.jar");
    write_atomic(&dest, b"content").unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"content");
    // The `.part` file must not survive the rename.
    assert!(!dest.with_extension("jar.part").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_file_without_an_extension_still_gets_a_part() {
    let dir = folder("atomic-without-ext");
    let dest = dir.join("LICENSE");
    write_atomic(&dest, b"text").unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"text");
    assert!(!dir.join("LICENSE.part").exists());
    std::fs::remove_dir_all(&dir).ok();
}

/// A CDN that returns an error page over HTTP 200 is caught here, not three
/// hours later as a NeoForge crash.
#[tokio::test]
async fn content_that_does_not_match_its_checksum_is_not_written() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/jei.jar", b"<html>404 not found</html>");
    let dir = folder("wrong-checksum");
    let dest = dir.join("jei.jar");

    let expected = sha1(b"the real jar");
    let error = client()
        .to_file(&server.url("/jei.jar"), &dest, Check::Full(&expected))
        .await
        .expect_err("the checksum does not match");

    assert!(format!("{error:#}").contains("SHA-1"), "{error:#}");
    assert!(!dest.exists(), "a wrong file was written to disk");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn matching_content_is_written_and_announced_as_downloaded() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/jei.jar", b"the real jar");
    let dir = folder("matching-checksum");
    let dest = dir.join("jei.jar");

    let outcome = client()
        .to_file(
            &server.url("/jei.jar"),
            &dest,
            Check::Full(&sha1(b"the real jar")),
        )
        .await
        .unwrap();

    assert_eq!(outcome, Fetched::Downloaded);
    assert_eq!(std::fs::read(&dest).unwrap(), b"the real jar");
    std::fs::remove_dir_all(&dir).ok();
}

/// Resuming an interrupted install picks up where it left off, which
/// matters when there are 2,500 asset objects left.
#[tokio::test]
async fn an_already_matching_file_is_not_redownloaded() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/jei.jar", b"the real jar");
    let dir = folder("already-there");
    let dest = dir.join("jei.jar");
    write_atomic(&dest, b"the real jar").unwrap();

    let outcome = client()
        .to_file(
            &server.url("/jei.jar"),
            &dest,
            Check::Full(&sha1(b"the real jar")),
        )
        .await
        .unwrap();

    assert_eq!(outcome, Fetched::AlreadyPresent);
    assert_eq!(server.calls("/jei.jar"), 0, "the network was used");
    std::fs::remove_dir_all(&dir).ok();
}

/// A file present but corrupted must be redownloaded: that's the whole
/// point of recomputing the digest of what gets executed.
#[tokio::test]
async fn an_altered_file_is_downloaded_again() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/jei.jar", b"the real jar");
    let dir = folder("altered");
    let dest = dir.join("jei.jar");
    write_atomic(&dest, b"something else").unwrap();

    let outcome = client()
        .to_file(
            &server.url("/jei.jar"),
            &dest,
            Check::Full(&sha1(b"the real jar")),
        )
        .await
        .unwrap();

    assert_eq!(outcome, Fetched::Downloaded);
    assert_eq!(std::fs::read(&dest).unwrap(), b"the real jar");
    std::fs::remove_dir_all(&dir).ok();
}

/// Recomputing the SHA-1 of 800 MB of assets on every launch would reread
/// the whole disk to almost never find anything: size is enough as a first
/// barrier.
#[tokio::test]
async fn the_quick_check_relies_only_on_size() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/object", b"12345");
    let dir = folder("quick");
    let dest = dir.join("object");
    // Same size, different content: the quick check accepts it.
    write_atomic(&dest, b"abcde").unwrap();

    let sum = sha1(b"12345");
    let outcome = client()
        .to_file(
            &server.url("/object"),
            &dest,
            Check::Quick { sum: &sum, size: 5 },
        )
        .await
        .unwrap();

    assert_eq!(outcome, Fetched::AlreadyPresent);
    std::fs::remove_dir_all(&dir).ok();
}

/// Absent a digest, size is the only check possible. It's enough to rule
/// out an error page served over HTTP 200, by far the most frequent case.
#[tokio::test]
async fn without_a_checksum_the_announced_size_acts_as_the_check() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/mod.jar", b"<html>oops</html>");
    let dir = folder("size");
    let dest = dir.join("mod.jar");

    let error = client()
        .to_file(&server.url("/mod.jar"), &dest, Check::Size(4096))
        .await
        .expect_err("the size does not match");

    let text = format!("{error:#}");
    assert!(text.contains("4096 announced"), "{text}");
    assert!(!dest.exists());
    std::fs::remove_dir_all(&dir).ok();
}

/// When the source publishes neither a digest nor a size, presence is all
/// that can be established — and that beats redownloading forever.
#[tokio::test]
async fn without_anything_published_presence_is_enough() {
    let server = mc_testkit::Server::new().await;
    server.bytes("/unknown.jar", b"content");
    let dir = folder("presence");
    let dest = dir.join("unknown.jar");

    let first = client()
        .to_file(&server.url("/unknown.jar"), &dest, Check::Presence)
        .await
        .unwrap();
    let second = client()
        .to_file(&server.url("/unknown.jar"), &dest, Check::Presence)
        .await
        .unwrap();

    assert_eq!(first, Fetched::Downloaded);
    assert_eq!(second, Fetched::AlreadyPresent);
    assert_eq!(server.calls("/unknown.jar"), 1);
    std::fs::remove_dir_all(&dir).ok();
}

/// **What `read_off_thread` returns, and not merely that it doesn't fail.**
///
/// Three mutants survived it: returning an empty vector, `[0]`, `[1]`. No
/// test looked at the CONTENT, and all three read perfectly fine at
/// runtime — `mc-news` uses it to reread the offline feed copy, and an empty
/// or one-byte feed is handled exactly like an unreadable feed: the news
/// page would go silent, without an error.
///
/// The content carries accented characters and a null byte: the former
/// because real feed content is written in French, the latter because a
/// read going through a string would stop right there.
#[tokio::test]
async fn an_off_thread_read_returns_the_file_bytes() {
    let dir = folder("off-thread-read");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("feed.json");
    let expected: Vec<u8> = b"{\"billets\":[]} \xc3\xa9pingl\xc3\xa9e\x00fin".to_vec();
    std::fs::write(&source, &expected).unwrap();

    let read = super::read_off_thread(&source).await.unwrap();

    assert_eq!(read, expected);
    std::fs::remove_dir_all(&dir).ok();
}

/// A missing file returns an ERROR, not an empty vector.
///
/// The distinction carries all of `mc-news`'s offline handling: "the copy
/// doesn't exist" calls for going to the network, "the copy is empty" would
/// be a feed with no posts, displayed as such.
#[tokio::test]
async fn an_off_thread_read_of_a_missing_file_fails() {
    let dir = folder("off-thread-read-missing");
    std::fs::create_dir_all(&dir).unwrap();

    let error = super::read_off_thread(&dir.join("nowhere.json")).await;

    assert!(error.is_err(), "a missing file must fail");
    std::fs::remove_dir_all(&dir).ok();
}

/// The write-side counterpart: what was written reads back identically.
///
/// `write_off_thread` takes its bytes by value and hands them to a detached
/// task; nothing in the suite checked that they arrived whole on the other
/// side.
#[tokio::test]
async fn an_off_thread_write_places_exactly_what_it_is_given() {
    let dir = folder("off-thread-write");
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("copy.json");
    let bytes: Vec<u8> = b"off-thread write \xc3\xa9\x00".to_vec();

    super::write_off_thread(&dest, bytes.clone()).await.unwrap();

    assert_eq!(std::fs::read(&dest).unwrap(), bytes);
    std::fs::remove_dir_all(&dir).ok();
}
