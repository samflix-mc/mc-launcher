use crate::fixtures::{Project, Version, Workshop, jar, publish};
use crate::resolve::registry::Registry;
use crate::resolve::request::Request;
use crate::{Origin, jar::Side};

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn registry(workshop: &Workshop, server: &mc_testkit::Server) -> Registry {
    Registry::for_fixtures(workshop.root.join("cache"), &server.base()).unwrap()
}

/// A CurseForge project, served by the site's public API.
///
/// Three routes: cfwidget to recover the id from a slug, the file list, and
/// the isolated file a pinned build goes through.
fn publish_core(server: &mc_testkit::Server, id: u32, slug: &str) {
    server.json(
        &format!("/widget/{slug}"),
        &format!(r#"{{"id":{id},"title":"{slug}"}}"#),
    );
    server.json(
        &format!("/web/mods/{id}/files"),
        &format!(
            r#"{{"data":[{{"id":7,"fileName":"{slug}-core.jar","displayName":"1.0",
                 "fileLength":1,"releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
                 "gameVersions":["1.21.1","NeoForge"]}}],"pagination":{{"totalCount":1}}}}"#
        ),
    );
    server.json(&format!("/web/mods/{id}/dependencies"), r#"{"data":[]}"#);
    server.json(
        &format!("/web/mods/{id}/files/7"),
        &format!(
            r#"{{"data":{{"id":7,"fileName":"{slug}-core.jar","displayName":"1.0",
             "fileLength":1,"releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
             "gameVersions":["1.21.1","NeoForge"]}}}}"#
        ),
    );
}

/// A numeric identifier can only come from CurseForge: Modrinth names its
/// projects with slugs. Offering it to Modrinth would make a request doomed
/// to fail for every resolved dependency — a hundred mods, a hundred round
/// trips.
#[tokio::test]
async fn a_numeric_identifier_is_not_offered_to_modrinth() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-numeric");
    publish_core(&server, 42, "jei");

    let found = registry(&workshop, &server)
        .candidates("42", None, MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found[0].file_name, "jei-core.jar");
    assert_eq!(
        server.calls("/project/42"),
        0,
        "Modrinth was queried for a numeric identifier"
    );
}

/// When the source is named, it's the only one consulted. A manifest that
/// writes "modrinth" has a reason to — the digest, the license, the
/// client/server split — and silently falling back to CurseForge would give
/// it a different file than the one it asked for.
#[tokio::test]
async fn a_named_source_is_not_doubled_by_the_other() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-source");
    // Modrinth doesn't know this project; CurseForge does.
    publish_core(&server, 42, "jei");

    let found = registry(&workshop, &server)
        .candidates("jei", Some(Origin::Modrinth), MC, LOADER)
        .await
        .unwrap();

    assert!(
        found.is_empty(),
        "CurseForge answered a request addressed to Modrinth: {found:?}"
    );
    assert_eq!(server.calls("/widget/jei"), 0);
}

/// Without a named source, Modrinth goes first; if it finds nothing, it's
/// CurseForge that answers.
#[tokio::test]
async fn without_a_named_source_modrinth_comes_before_curseforge() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-default");
    let content = jar("jei", &[]);
    server.bytes("/jei.jar", &content);
    publish(
        &server,
        &Project::new("jei").version(Version::new("19.21", &server.url("/jei.jar"), &content)),
    );
    publish_core(&server, 42, "jei");

    let found = registry(&workshop, &server)
        .candidates("jei", None, MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found[0].origin, Origin::Modrinth);
    assert_eq!(server.calls("/widget/jei"), 0);
}

/// A build pinned by a numeric identifier goes to CurseForge without going
/// through Modrinth: the number means nothing to it.
#[tokio::test]
async fn a_numeric_pinned_build_goes_to_curseforge() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-pinned-num");
    publish_core(&server, 42, "jei");
    // An isolated build is looked up by digest search: the single-file route
    // would also require the project's id.
    server.json(
        "/mods/files",
        r#"{"data":[{"id":7,"modId":42,"displayName":"1.0","fileName":"jei-core.jar",
             "releaseType":1,"fileDate":"2026-01-01T00:00:00Z",
             "downloadUrl":"https://exemple.invalid/jei.jar","fileLength":1,
             "gameVersions":["1.21.1","NeoForge"],"hashes":[],"dependencies":[]}]}"#,
    );

    let mut request = Request::new("42");
    request.file = Some("7".into());
    request.side = Some(Side::Both);

    let found = registry(&workshop, &server)
        .pinned(&request, MC, LOADER)
        .await
        .expect("the Core API answers")
        .expect("the pinned build exists");

    assert_eq!(found.file_name, "jei-core.jar");
    assert_eq!(
        server.calls("/version/7"),
        0,
        "Modrinth was queried for a numeric build"
    );
}

/// The other half of the rule: a request addressed to CurseForge doesn't go
/// through Modrinth either, even when named by a slug. The two conditions
/// each count on their own.
#[tokio::test]
async fn a_slug_addressed_to_curseforge_does_not_go_through_modrinth() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-slug-cf");
    publish_core(&server, 42, "jei");

    let found = registry(&workshop, &server)
        .candidates("jei", Some(Origin::CurseForge), MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found[0].file_name, "jei-core.jar");
    assert_eq!(
        server.calls("/project/jei"),
        0,
        "Modrinth was queried for a request addressed to CurseForge"
    );
}

/// A build pinned by a **non**-numeric identifier comes from Modrinth: it's
/// the one that names its versions this way. Sending it to CurseForge would
/// look for a file number where there's only a slug, and the pin would be
/// declared missing even though it exists.
#[tokio::test]
async fn a_non_numeric_pinned_build_goes_to_modrinth() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-pinned-slug");

    let mut request = Request::new("jei");
    request.file = Some("eyZ2YBGT".into());
    request.side = Some(Side::Both);

    // No source knows this build: what's being verified here is who the
    // question is addressed to.
    let _ = registry(&workshop, &server)
        .pinned(&request, MC, LOADER)
        .await;

    assert_eq!(
        server.calls("/version/eyZ2YBGT"),
        1,
        "Modrinth wasn't consulted for a build only it names this way"
    );
}

/// Without a pinned build, there's nothing to look for: the request follows
/// its ordinary course. Returning `None` for a build that *is* pinned would
/// fall back to the latest version — exactly what pinning is meant to
/// prevent.
#[tokio::test]
async fn a_request_without_pinning_does_not_look_for_a_build() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("search-no-pinned");

    let found = registry(&workshop, &server)
        .pinned(&Request::new("jei"), MC, LOADER)
        .await
        .unwrap();

    assert!(found.is_none());
    assert_eq!(server.calls("/mods/files"), 0);
}
