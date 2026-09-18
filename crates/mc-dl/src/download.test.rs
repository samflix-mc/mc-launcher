use super::{Downloader, backoff};

/// The backoff decides what a stuttering source costs: too short, and we
/// retry before the CDN has recovered; too long, and a modpack of a
/// thousand files spends its minutes waiting. The calculation is therefore
/// worth checking, since only a clock inside a test could otherwise tell.
#[test]
fn the_first_attempt_starts_without_waiting_and_the_next_ones_wait() {
    assert_eq!(backoff(0), std::time::Duration::ZERO);
    assert_eq!(backoff(1), std::time::Duration::from_millis(400));
    assert_eq!(backoff(2), std::time::Duration::from_millis(800));
}

fn client() -> Downloader {
    Downloader::new(crate::USER_AGENT).expect("the client builds")
}

/// Modrinth explicitly requires an identifiable agent and rate-limits
/// anonymous agents more severely; CurseForge logs it along with the key.
/// That's what makes abuse traceable back to us.
#[tokio::test]
async fn each_request_announces_the_launcher_agent() {
    let server = mc_testkit::Server::new().await;
    server.json("/v2/project/jei", r#"{"slug":"jei"}"#);

    client()
        .bytes(&server.url("/v2/project/jei"))
        .await
        .unwrap();

    let received = &server.received()[0];
    assert_eq!(received.method, "GET");
    assert_eq!(
        received.header("user-agent"),
        Some(crate::USER_AGENT),
        "agent announced: {:?}",
        received.header("user-agent")
    );
}

#[tokio::test]
async fn the_response_body_is_returned_unchanged() {
    let server = mc_testkit::Server::new().await;
    server.json("/list", r#"{"versions":["21.1.250"]}"#);

    let bytes = client().bytes(&server.url("/list")).await.unwrap();
    assert_eq!(bytes, br#"{"versions":["21.1.250"]}"#);
}

/// A CDN's 5xx errors pass within a few seconds, and a modpack makes
/// thousands of requests: giving up on the first failure would make an
/// install impossible over a two-second outage.
#[tokio::test]
async fn a_transient_failure_is_retried() {
    let server = mc_testkit::Server::new().await;
    server.fails_then("/flaky", 2, r#"{"ok":true}"#);

    let bytes = client().bytes(&server.url("/flaky")).await.unwrap();

    assert_eq!(bytes, br#"{"ok":true}"#);
    assert_eq!(server.calls("/flaky"), 3, "the retry did not happen");
}

/// Three attempts, no more: past that point, the error must surface with
/// the offending URL rather than extend the wait.
#[tokio::test]
async fn a_persistent_failure_eventually_surfaces_with_the_url() {
    let server = mc_testkit::Server::new().await;
    server.code("/dead", 500);

    let error = client()
        .bytes(&server.url("/dead"))
        .await
        .expect_err("three failures");

    let text = format!("{error:#}");
    assert!(text.contains("/dead"), "{text}");
    assert!(text.contains("500"), "{text}");
    assert_eq!(server.calls("/dead"), 3);
}

/// The body of an error response often carries the explanation — "invalid
/// key", "quota exceeded". Losing it forces a guess.
#[tokio::test]
async fn the_error_body_accompanies_the_code() {
    let server = mc_testkit::Server::new().await;
    server.code_with("/refused", 403, r#"{"error":"invalid API key"}"#);

    let error = client()
        .bytes(&server.url("/refused"))
        .await
        .expect_err("403");

    assert!(
        format!("{error:#}").contains("invalid API key"),
        "{error:#}"
    );
}

#[tokio::test]
async fn an_unknown_path_surfaces_a_404() {
    let server = mc_testkit::Server::new().await;
    let error = client()
        .bytes(&server.url("/nowhere"))
        .await
        .expect_err("404");
    assert!(format!("{error:#}").contains("404"), "{error:#}");
}

#[test]
fn the_client_is_shareable_as_is() {
    // `client()` is exposed for calls that need their own headers — the
    // CurseForge key, the Microsoft token.
    let dl = client();
    assert!(std::ptr::eq(dl.client(), dl.client()));
}

/// A 404 returns an [`Absent`], not a transport error.
///
/// That's what lets a caller distinguish "this host doesn't publish this
/// resource" from "this host said nothing". Confusing the two made a news
/// page show an error on every host that doesn't publish one yet.
#[tokio::test]
async fn a_404_is_an_absent_not_a_failure() {
    let server = mc_testkit::Server::new().await;
    let dl = client();

    let error = dl
        .bytes(&server.url("/nothing-at-all.json"))
        .await
        .expect_err("404");

    assert!(
        error.downcast_ref::<super::Absent>().is_some(),
        "a 404 must return an Absent, not \"{error}\""
    );
    assert!(error.to_string().contains("nothing-at-all.json"));
}

/// And it is NOT retried.
///
/// Three attempts on an address that answered "it's not there" cost two
/// round-trips and two backoff waits for the same answer. The test checks
/// this through TIME, the only thing that observes it from the outside:
/// with retries, the backoffs add up.
#[tokio::test]
async fn an_absent_is_not_retried() {
    let server = mc_testkit::Server::new().await;
    let dl = client();

    let start = std::time::Instant::now();
    let _ = dl.bytes(&server.url("/absent.json")).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "an absent retried three times: {elapsed:?}"
    );
}
