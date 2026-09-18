use super::super::CurseForgeWeb;
use super::cache_delay;
use std::sync::Arc;

/// The second attempt only makes sense if it gives cfwidget time to build
/// its cache: retrying right away would just get another 202, and the
/// project would be declared missing even though it exists.
#[test]
fn the_second_attempt_gives_the_cache_time_to_fill() {
    assert_eq!(cache_delay(0), std::time::Duration::ZERO);
    assert_eq!(cache_delay(1), std::time::Duration::from_secs(3));
}

fn client(server: &mc_testkit::Server) -> CurseForgeWeb {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForgeWeb::with_bases(dl, &server.base(), &server.url("/widget"))
}

#[tokio::test]
async fn a_slug_yields_its_id_and_name() {
    let server = mc_testkit::Server::new().await;
    server.json(
        "/widget/jei",
        r#"{"id":238222,"title":"Just Enough Items"}"#,
    );

    let found = client(&server).project_id("jei").await.unwrap();
    assert_eq!(found, Some((238222, "Just Enough Items".to_string())));
}

/// cfwidget returns 202 while it builds its cache for a project it has never
/// seen. Giving up on the first call would make every recent mod
/// unfindable without an API key.
#[tokio::test]
async fn a_202_from_cfwidget_triggers_a_second_attempt() {
    let server = mc_testkit::Server::new().await;
    server.code("/widget/new", 202);

    let found = client(&server).project_id("new").await.unwrap();

    assert_eq!(found, None, "no result after two attempts");
    assert_eq!(server.calls("/widget/new"), 2);
}

#[tokio::test]
async fn an_unknown_project_from_cfwidget_yields_nothing() {
    let server = mc_testkit::Server::new().await;
    server.code("/widget/unknown", 404);

    assert_eq!(client(&server).project_id("unknown").await.unwrap(), None);
    // A 404 is final: no point retrying.
    assert_eq!(server.calls("/widget/unknown"), 1);
}

/// cfwidget is a volunteer third-party service: a response that no longer
/// has the expected shape must count as "nothing found", not stop the
/// install.
#[tokio::test]
async fn an_unreadable_response_counts_as_absence() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/broken", r#"{"identifiant":238222}"#);

    assert_eq!(client(&server).project_id("broken").await.unwrap(), None);
}

/// A numeric id comes from an already-resolved dependency: the readable name
/// isn't essential, and skipping it saves a call to a third-party service.
#[tokio::test]
async fn a_numeric_id_resolves_without_a_call() {
    let server = mc_testkit::Server::new().await;

    let found = client(&server).resolve_project("238222").await.unwrap();

    assert_eq!(found, Some((238222, "project 238222".to_string())));
    assert!(server.received().is_empty(), "{:?}", server.received());
}

#[tokio::test]
async fn a_slug_does_go_through_cfwidget() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/jei", r#"{"id":1,"title":"JEI"}"#);

    let found = client(&server).resolve_project("jei").await.unwrap();
    assert_eq!(found, Some((1, "JEI".to_string())));
}
