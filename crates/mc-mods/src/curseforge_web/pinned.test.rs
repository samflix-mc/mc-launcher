use super::super::CurseForgeWeb;
use std::sync::Arc;

fn client(server: &mc_testkit::Server) -> CurseForgeWeb {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForgeWeb::with_bases(dl, &server.base(), &server.url("/widget"))
}

/// What the route actually renders: the object **wrapped in `data`**, like
/// the lists. The suite used to serve the bare object, which validated a
/// contract the API doesn't honor — and let a deserialization failure slip
/// through in production on every pinned build.
const FILE: &str = r#"{"data":{"id":5001,"fileName":"jei-19.jar","displayName":"JEI 19",
                          "fileLength":2048,"releaseType":2,
                          "dateCreated":"2026-01-01T00:00:00Z",
                          "gameVersions":["1.21.1","NeoForge"]}}"#;

/// A pinned build isn't filtered by the API: it's rendered as-is, with its
/// channel, and that's what makes an install reproducible.
#[tokio::test]
async fn a_pinned_build_is_returned_as_is() {
    let server = mc_testkit::Server::new().await;
    server.json(
        "/widget/jei",
        r#"{"id":238222,"title":"Just Enough Items"}"#,
    );
    server.json("/mods/238222/files/5001", FILE);

    let found = client(&server)
        .candidate_by_file("jei", "5001")
        .await
        .unwrap()
        .expect("the build exists");

    assert_eq!(found.version_id, "5001");
    assert_eq!(found.channel, crate::Channel::Beta);
    assert_eq!(found.name, "Just Enough Items");
    assert!(found.page_url.as_deref().unwrap().contains("jei"));
}

#[tokio::test]
async fn a_deleted_pinned_build_returns_nothing() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/jei", r#"{"id":238222,"title":"JEI"}"#);
    server.code("/mods/238222/files/9999", 404);

    assert!(
        client(&server)
            .candidate_by_file("jei", "9999")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn an_unknown_project_yields_no_build() {
    let server = mc_testkit::Server::new().await;
    server.code("/widget/unknown", 404);

    assert!(
        client(&server)
            .candidate_by_file("unknown", "1")
            .await
            .unwrap()
            .is_none()
    );
}

/// Without a key, keyword search is closed: only a `modId` that's also the
/// project's slug can succeed.
#[tokio::test]
async fn search_by_mod_id_goes_through_the_slug() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/bookshelf", r#"{"id":42,"title":"Bookshelf"}"#);
    server.json(
        "/mods/42/files",
        r#"{"data":[{"id":1,"fileName":"bookshelf.jar","displayName":"Bookshelf",
                     "fileLength":1,"releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
                     "gameVersions":["1.21.1","NeoForge"]}],"pagination":{"totalCount":1}}"#,
    );
    server.json("/mods/42/dependencies", r#"{"data":[]}"#);

    let found = client(&server)
        .find_by_mod_id("bookshelf", "1.21.1", "neoforge")
        .await
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].slug, "bookshelf");
}

/// The exact regression seen in installs: the route renders the object
/// wrapped, the code read it bare, deserialization failed, and the failure
/// was swallowed — "build 5513549 pinned for 882495: missing", for a build
/// that genuinely existed.
#[tokio::test]
async fn an_object_without_its_envelope_is_not_mistaken_for_an_absence() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/jei", r#"{"id":238222,"title":"JEI"}"#);
    // The bare object, as the suite used to serve it: it's not what the API
    // renders, and the code must no longer put up with it.
    server.json(
        "/mods/238222/files/5001",
        r#"{"id":5001,"fileName":"jei.jar","displayName":"JEI","fileLength":1,
            "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z","gameVersions":[]}"#,
    );

    assert!(
        client(&server)
            .candidate_by_file("jei", "5001")
            .await
            .unwrap()
            .is_none(),
        "an out-of-contract response must not be mistaken for a candidate"
    );
}

/// A numeric id needs no call to cfwidget: it's already the project's id.
/// That's the case for every mod a lockfile pins by its CurseForge id.
#[tokio::test]
async fn a_numeric_project_resolves_without_the_widget() {
    let server = mc_testkit::Server::new().await;
    server.json("/mods/882495/files/5513549", FILE);

    let found = client(&server)
        .candidate_by_file("882495", "5513549")
        .await
        .unwrap()
        .expect("the build exists");

    assert_eq!(found.version_id, "5001");
    assert_eq!(server.calls("/widget/882495"), 0);
}
