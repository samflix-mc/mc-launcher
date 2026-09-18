use super::{IMAGE_MAX, base64, feed_url, fetch_image, load, mime_type, read, to_data_url};

struct Workshop {
    root: std::path::PathBuf,
}

impl Workshop {
    fn new(name: &str) -> Workshop {
        let root = std::env::temp_dir().join(format!(
            "mc-news-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Workshop { root }
    }
}

impl Drop for Workshop {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

const FEED: &str = "https://mc-launcher.ggy.info/pack/news.json";

fn raw_feed(posts: &str) -> String {
    format!(r#"{{"schema":1,"posts":[{posts}]}}"#)
}

fn a_post(id: &str, date: &str) -> String {
    format!(r#"{{"id":"{id}","title":"Title","date":"{date}","body":"Hello."}}"#)
}

/// The feed's address is DERIVED from the pack's, never recopied.
///
/// The same pattern as `lock_url_for`: recopying the three published
/// addresses would recreate the flaw their comment describes — a
/// preproduction that downloads production's content, and therefore proves
/// none of what it's supposed to prove.
#[test]
fn the_feed_address_derives_from_the_pack_s() {
    assert_eq!(
        feed_url("https://mc-launcher.ggy.info/pack/samflix.json"),
        "https://mc-launcher.ggy.info/pack/news.json"
    );
    assert_eq!(
        feed_url("https://mc-launcher-dev.ggy.info/pack/samflix.json"),
        "https://mc-launcher-dev.ggy.info/pack/news.json"
    );
}

/// A faulty post is discarded ALONE.
///
/// That's the difference between "the news page has a gap" and "the news
/// page is empty", and the second reads like a launcher failure — when the
/// cause is a comma in a file we don't control at the same pace as the code.
#[test]
fn a_faulty_post_does_not_take_down_the_others() {
    let raw = raw_feed(&format!(
        "{},{},{}",
        a_post("good-1", "2026-09-01T00:00:00Z"),
        a_post("broken-date", "yesterday"),
        a_post("good-2", "2026-09-02T00:00:00Z"),
    ));

    let feed = read(raw.as_bytes(), FEED).expect("the feed parses");

    assert_eq!(feed.posts.len(), 2, "{:?}", feed.discarded);
    assert_eq!(feed.discarded.len(), 1);
    assert!(
        feed.discarded[0].contains("broken-date"),
        "{:?}",
        feed.discarded
    );
}

#[test]
fn an_empty_title_discards_the_post() {
    let raw = raw_feed(r#"{"id":"empty","title":"  ","date":"2026-09-01T00:00:00Z","body":"x"}"#);
    let feed = read(raw.as_bytes(), FEED).unwrap();
    assert!(feed.posts.is_empty());
    assert_eq!(feed.discarded.len(), 1);
}

/// An image from elsewhere doesn't discard the post: it's simply ignored.
/// Losing an entire post because its illustration is misplaced would be
/// disproportionate.
#[test]
fn an_image_from_elsewhere_does_not_lose_the_post() {
    let raw = raw_feed(
        r#"{"id":"x","title":"T","date":"2026-09-01T00:00:00Z","image":"https://evil.example/i.webp","body":"x"}"#,
    );
    let feed = read(raw.as_bytes(), FEED).unwrap();

    assert_eq!(feed.posts.len(), 1);
    assert_eq!(feed.posts[0].image, None);
    assert_eq!(feed.discarded.len(), 1);
}

#[test]
fn a_relative_image_becomes_absolute() {
    let raw = raw_feed(
        r#"{"id":"x","title":"T","date":"2026-09-01T00:00:00Z","image":"s3.webp","body":"x"}"#,
    );
    let feed = read(raw.as_bytes(), FEED).unwrap();
    assert_eq!(
        feed.posts[0].image.as_deref(),
        Some("https://mc-launcher.ggy.info/pack/s3.webp")
    );
}

#[test]
fn an_unreadable_feed_returns_an_error() {
    assert!(read(b"{not JSON", FEED).is_err());
}

// --- Network and fallback ---------------------------------------------

#[tokio::test]
async fn the_feed_is_fetched_and_cached() {
    let workshop = Workshop::new("network-feed");
    let server = mc_testkit::Server::new().await;
    let raw = raw_feed(&a_post("a", "2026-09-01T00:00:00Z"));
    server.json("/pack/news.json", &raw);
    let dl = mc_dl::Downloader::new("test").unwrap();

    let feed = load(
        &format!("{}/pack/samflix.json", server.base()),
        &workshop.root,
        &dl,
    )
    .await
    .expect("the feed loads");

    assert_eq!(feed.posts.len(), 1);
    assert!(!feed.offline);
    assert!(workshop.root.join("news.json").is_file(), "no copy");
}

/// Without network, the copy takes over and says so. A day-old news page is
/// better than an empty page; a page that lied about its freshness would be
/// worse than either.
#[tokio::test]
async fn without_network_the_copy_takes_over_and_says_so() {
    let workshop = Workshop::new("offline-feed");
    let raw = raw_feed(&a_post("a", "2026-09-01T00:00:00Z"));
    std::fs::write(workshop.root.join("news.json"), &raw).unwrap();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let feed = load("http://127.0.0.1:1/pack/samflix.json", &workshop.root, &dl)
        .await
        .expect("the copy serves");

    assert_eq!(feed.posts.len(), 1);
    assert!(
        feed.offline,
        "the window would believe the feed is up to date"
    );
}

/// Without network AND without a copy, a clear-cut error: it's the only
/// situation where there's nothing to show, and the page must say so rather
/// than display an emptiness that would be mistaken for "no news".
#[tokio::test]
async fn without_network_or_copy_the_error_is_clear_cut() {
    let workshop = Workshop::new("no-feed");
    let dl = mc_dl::Downloader::new("test").unwrap();

    assert!(
        load("http://127.0.0.1:1/pack/samflix.json", &workshop.root, &dl)
            .await
            .is_err()
    );
}

/// An unreadable response does NOT replace a valid copy. Without this
/// precaution, a host serving an HTML error page with HTTP 200 would
/// overwrite the last known feed with nothing.
#[tokio::test]
async fn an_unreadable_response_does_not_overwrite_the_copy() {
    let workshop = Workshop::new("html-feed");
    let good = raw_feed(&a_post("a", "2026-09-01T00:00:00Z"));
    std::fs::write(workshop.root.join("news.json"), &good).unwrap();

    let server = mc_testkit::Server::new().await;
    server.json("/pack/news.json", "<html>maintenance</html>");
    let dl = mc_dl::Downloader::new("test").unwrap();

    let _ = load(
        &format!("{}/pack/samflix.json", server.base()),
        &workshop.root,
        &dl,
    )
    .await;

    let after = std::fs::read(workshop.root.join("news.json")).unwrap();
    assert_eq!(after, good.as_bytes(), "the valid copy was overwritten");
}

// --- Images --------------------------------------------------------------

/// The type is deduced from the BYTES and not the extension: an extension
/// comes from the URL, hence from the host, hence from something we don't
/// control at the same pace as the code.
#[test]
fn the_type_is_deduced_from_the_bytes() {
    assert_eq!(mime_type(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
    assert_eq!(mime_type(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    assert_eq!(mime_type(b"GIF89a....."), Some("image/gif"));
    assert_eq!(mime_type(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));

    assert_eq!(mime_type(b"<html>"), None);
    assert_eq!(mime_type(b""), None);
    // "RIFF" without "WEBP" is a WAV: an image must be an image.
    assert_eq!(mime_type(b"RIFF\0\0\0\0WAVEfmt "), None);
}

/// The bound exists because `Downloader::bytes` has NONE and accumulates the
/// entire body in memory, under a three-hundred-second timeout. A faulty
/// host would serve a huge file, and the launcher would grow until the
/// system kills it — with no message.
#[tokio::test]
async fn an_oversized_image_is_refused() {
    let workshop = Workshop::new("big-image");
    let server = mc_testkit::Server::new().await;
    let mut huge = b"\x89PNG\r\n\x1a\n".to_vec();
    huge.resize(IMAGE_MAX + 1, 0);
    server.bytes("/big.png", &huge);
    let dl = mc_dl::Downloader::new("test").unwrap();

    let error = fetch_image(&server.url("/big.png"), &workshop.root, &dl)
        .await
        .expect_err("an image over 512 KiB must be refused");
    assert!(format!("{error:#}").contains("512"), "{error:#}");
}

/// Whatever isn't an image isn't cached as if it were one: an HTML error
/// page served in place of an illustration would become a `data:text/html`
/// loaded into the window.
#[tokio::test]
async fn whatever_is_not_an_image_is_refused() {
    let workshop = Workshop::new("fake-image");
    let server = mc_testkit::Server::new().await;
    server.bytes("/fake.png", b"<html>404</html>");
    let dl = mc_dl::Downloader::new("test").unwrap();

    assert!(
        fetch_image(&server.url("/fake.png"), &workshop.root, &dl)
            .await
            .is_err()
    );
}

/// The `data:` is only built ON DEMAND, from a file.
///
/// Keeping images as base64 in the JSON cache would mean a feed of eight
/// illustrated posts weighed three and a half megabytes, re-read and
/// re-serialized on every page open — for a render that only shows one
/// before you scroll.
#[tokio::test]
async fn the_image_is_a_file_and_the_data_url_is_built_on_demand() {
    let workshop = Workshop::new("cached-image");
    let server = mc_testkit::Server::new().await;
    server.bytes("/i.png", b"\x89PNG\r\n\x1a\ncontent");
    let dl = mc_dl::Downloader::new("test").unwrap();

    let path = fetch_image(&server.url("/i.png"), &workshop.root, &dl)
        .await
        .expect("the image is fetched");

    assert!(path.is_file());
    // A FILE, not base64 in a JSON.
    assert_eq!(std::fs::read(&path).unwrap(), b"\x89PNG\r\n\x1a\ncontent");

    let data = to_data_url(&path).expect("the data: is built");
    assert!(data.starts_with("data:image/png;base64,"), "{data}");
}

/// Two posts might each carry an `image.webp`: the cache name is the URL's
/// digest, without which the second would overwrite the first.
#[tokio::test]
async fn two_images_with_the_same_name_do_not_collide() {
    let workshop = Workshop::new("image-collision");
    let server = mc_testkit::Server::new().await;
    server.bytes("/a/i.png", b"\x89PNG\r\n\x1a\nAAA");
    server.bytes("/b/i.png", b"\x89PNG\r\n\x1a\nBBB");
    let dl = mc_dl::Downloader::new("test").unwrap();

    let one = fetch_image(&server.url("/a/i.png"), &workshop.root, &dl)
        .await
        .unwrap();
    let two = fetch_image(&server.url("/b/i.png"), &workshop.root, &dl)
        .await
        .unwrap();

    assert_ne!(one, two);
    assert!(std::fs::read(&one).unwrap().ends_with(b"AAA"));
    assert!(std::fs::read(&two).unwrap().ends_with(b"BBB"));
}

// --- base64, against the standard's test vectors --------------------------

/// The seven vectors from RFC 4648, section 10.
///
/// ## Why they matter more than a "looks like base64" test
///
/// This encoding is written by hand — thirty lines rather than one more
/// crate in a binary we ship. The price of that choice is that no proven
/// library covers it: it's on us to do it.
///
/// The seven vectors are exactly designed for that. They walk through the
/// THREE padding cases — zero, one, and two "=" signs — and enough bit
/// patterns that a reversed shift, an "or" turned "and", or a mask shifted
/// by one notch changes the output. Without them, twenty mutations of these
/// operators survived: the code produced something, and nothing said it was
/// the right something.
#[test]
fn base64_follows_the_standard() {
    for (plain, expected) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(plain.as_bytes()), expected, "\"{plain}\"");
    }
}

/// High bytes pass too: an encoding that only worked on ASCII would be
/// useless here, where we encode PNGs and WebPs.
#[test]
fn base64_encodes_high_bytes() {
    assert_eq!(base64(&[0xFF, 0xFF, 0xFF]), "////");
    assert_eq!(base64(&[0x00, 0x00, 0x00]), "AAAA");
    assert_eq!(base64(&[0xFB, 0xFF, 0xBF]), "+/+/");
    // The start of a real PNG, so the test speaks to what we encode.
    assert_eq!(base64(b"\x89PNG\r\n\x1a\n"), "iVBORw0KGgo=");
}

// --- The bounds that were missing -----------------------------------------

/// Twelve bytes ARE the complete WebP signature: they're enough.
///
/// Eleven aren't — "WEBP" would be missing a byte. Testing both sides of the
/// bound is what fixes the operator: with only the "too short" case, `>` and
/// `>=` would produce the same thing.
#[test]
fn twelve_bytes_are_enough_to_recognize_a_webp() {
    let twelve = b"RIFF\0\0\0\0WEBP";
    assert_eq!(twelve.len(), 12);
    assert_eq!(mime_type(twelve), Some("image/webp"));
    assert_eq!(mime_type(&twelve[..11]), None);
}

/// A feed on a different schema still parses, AND says so.
///
/// The log alone wasn't enough: nobody opens it as long as nothing looks
/// broken, and that's precisely the case here — the posts display just
/// fine. So the discrepancy goes into the report, which the page already
/// shows.
#[test]
fn a_different_schema_parses_and_says_so() {
    let raw = format!(
        r#"{{"schema":99,"posts":[{}]}}"#,
        a_post("a", "2026-09-01T00:00:00Z")
    );

    let feed = read(raw.as_bytes(), FEED).expect("the feed still parses");

    assert_eq!(feed.posts.len(), 1, "the posts were lost");
    assert!(
        feed.discarded.iter().any(|e| e.contains("schema 99")),
        "the schema mismatch isn't reported anywhere: {:?}",
        feed.discarded
    );
}

/// And the right schema says nothing: a permanent warning ends up unread,
/// and takes the ones that mattered down with it.
#[test]
fn the_right_schema_says_nothing() {
    let raw = raw_feed(&a_post("a", "2026-09-01T00:00:00Z"));
    let feed = read(raw.as_bytes(), FEED).unwrap();
    assert!(feed.discarded.is_empty(), "{:?}", feed.discarded);
}

/// An image of EXACTLY the maximum size is accepted.
///
/// The counterpart of the test that refuses `IMAGE_MAX + 1`. Without both,
/// `>` and `>=` are equivalent — and an image exactly at the limit would be
/// refused with nothing having decided that.
#[tokio::test]
async fn an_image_at_the_maximum_size_is_accepted() {
    let workshop = Workshop::new("exact-size-image");
    let server = mc_testkit::Server::new().await;
    let mut exact = b"\x89PNG\r\n\x1a\n".to_vec();
    exact.resize(IMAGE_MAX, 0);
    server.bytes("/exact.png", &exact);
    let dl = mc_dl::Downloader::new("test").unwrap();

    let path = fetch_image(&server.url("/exact.png"), &workshop.root, &dl)
        .await
        .expect("an image exactly at the limit must pass");
    assert_eq!(std::fs::metadata(&path).unwrap().len() as usize, IMAGE_MAX);
}

/// An image already in cache isn't requested again. This is what makes the
/// host no longer count page opens — the only thing fetching really buys on
/// the privacy side.
#[tokio::test]
async fn an_image_already_cached_is_not_requested_again() {
    let workshop = Workshop::new("already-cached-image");
    let server = mc_testkit::Server::new().await;
    server.bytes("/i.png", b"\x89PNG\r\n\x1a\nX");
    let dl = mc_dl::Downloader::new("test").unwrap();
    let url = server.url("/i.png");

    fetch_image(&url, &workshop.root, &dl).await.unwrap();
    let after_one = server.received().len();
    fetch_image(&url, &workshop.root, &dl).await.unwrap();

    assert_eq!(
        server.received().len(),
        after_one,
        "the host was requested again"
    );
}

/// **The example in `docs/nouvelles.md` is read by the test suite, not just
/// by humans.**
///
/// This file is what we ask mc-content to implement: it's therefore a
/// contract, served to another repo. A documentation example that doesn't
/// parse is worse than no example at all — it costs whoever copies it an
/// afternoon, and nobody thinks to doubt it.
///
/// The test extracts the last ```` ```json ```` block from the document. The
/// first is in ```` ```jsonc ````, with comments, and doesn't parse — the
/// distinction between the two languages is deliberate and worth keeping.
#[test]
fn the_documentation_example_parses() {
    let document = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/nouvelles.md"
    ))
    .expect("docs/nouvelles.md reads from the crate");

    let example = document
        .rsplit_once("```json\n")
        .and_then(|(_, rest)| rest.split_once("\n```"))
        .map(|(block, _)| block)
        .expect("the document carries a ```json block");

    let feed = read(example.as_bytes(), "https://example.invalid/news.json")
        .expect("the documentation example parses");

    assert!(
        feed.discarded.is_empty(),
        "the example must not discard anything: {:?}",
        feed.discarded
    );
    assert_eq!(feed.posts.len(), 2, "both posts are retained");
    // The pinned one comes first, whatever the array's order.
    assert!(feed.posts[0].pinned);
    // And the body is a tree, never a string: that's the property under
    // which the CSP was loosened.
    assert!(!feed.posts[0].body.is_empty());
}

/// **A host with no feed opens an empty page, not an error message.**
///
/// The case isn't theoretical: as of September 18, 2026, all three hosts
/// return 404 on `news.json`. The previous behavior therefore showed an
/// error to every player, on a page where there was simply nothing to say.
///
/// And the feed is NOT marked offline: the host answered.
#[tokio::test]
async fn a_host_with_no_feed_renders_an_empty_page() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("no-feed-host");
    let dl = mc_dl::Downloader::new("test").expect("client");

    let feed = load(&server.url("/pack/samflix.json"), &workshop.root, &dl)
        .await
        .expect("a host with no feed is not an error");

    assert!(feed.posts.is_empty());
    assert!(!feed.offline, "the host answered: this isn't offline");
    assert!(
        feed.discarded.is_empty(),
        "nothing was discarded: there was nothing"
    );
}

/// And a cached copy does NOT resurrect a feed the host removed.
///
/// This is the half of the choice that needs defending: we could serve the
/// old feed. But a post removed from the host was removed by someone, and
/// showing it again because we keep a copy would disobey that action.
#[tokio::test]
async fn a_removed_feed_does_not_come_back_from_the_cache() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("removed-feed");
    let dl = mc_dl::Downloader::new("test").expect("client");

    // A leftover copy, from a feed that existed yesterday.
    std::fs::write(
        workshop.root.join("news.json"),
        br#"{"schema":1,"posts":[{"id":"old","title":"Removed","date":"2026-09-01T00:00:00Z","body":"."}]}"#,
    )
    .expect("copy");

    let feed = load(&server.url("/pack/samflix.json"), &workshop.root, &dl)
        .await
        .expect("a host with no feed is not an error");

    assert!(
        feed.posts.is_empty(),
        "the removed feed came back from the cache: {:?}",
        feed.posts
    );
}
