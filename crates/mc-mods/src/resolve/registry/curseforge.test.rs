use crate::fixtures::{Project, Version, Workshop, jar, publish};
use crate::resolve::registry::Registry;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn registry(workshop: &Workshop, server: &mc_testkit::Server) -> Registry {
    Registry::for_fixtures(workshop.root.join("cache"), &server.base()).unwrap()
}

/// A project served by the site's public API.
fn publish_web(server: &mc_testkit::Server, id: u32, slug: &str) {
    server.json(
        &format!("/widget/{slug}"),
        &format!(r#"{{"id":{id},"title":"{slug}"}}"#),
    );
    server.json(
        &format!("/web/mods/{id}/files"),
        &format!(
            r#"{{"data":[{{"id":2,"fileName":"{slug}-web.jar","displayName":"1.0",
                 "fileLength":1,"releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
                 "gameVersions":["1.21.1","NeoForge"]}}],"pagination":{{"totalCount":1}}}}"#
        ),
    );
    server.json(&format!("/web/mods/{id}/dependencies"), r#"{"data":[]}"#);
}

#[tokio::test]
async fn the_site_is_the_only_way_at_curseforge() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("registry-site");
    publish_web(&server, 42, "jei");

    let found = registry(&workshop, &server)
        .curseforge_any("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found[0].file_name, "jei-web.jar");
}

/// Modrinth is consulted before CurseForge for a `modId`: it publishes the
/// digests and the client/server split, which the site doesn't give.
#[tokio::test]
async fn modrinth_comes_before_curseforge_for_a_mod_id() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("registry-modid");
    let content = jar("bookshelf", &[]);
    server.bytes("/bookshelf.jar", &content);
    publish(
        &server,
        &Project::new("bookshelf").version(Version::new(
            "20.2.0",
            &server.url("/bookshelf.jar"),
            &content,
        )),
    );
    publish_web(&server, 42, "bookshelf");

    let found = registry(&workshop, &server)
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found[0].origin, crate::Origin::Modrinth);
    // The site wasn't bothered: Modrinth had the answer.
    assert_eq!(server.calls("/widget/bookshelf"), 0);
}

/// The site's keyword search is closed: a `modId` Modrinth doesn't know and
/// that isn't a project's slug gives nothing — and that's not an error, the
/// catch-up loop must be able to conclude.
#[tokio::test]
async fn a_mod_id_unknown_to_all_sources_gives_nothing() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("registry-unknown");

    let found = registry(&workshop, &server)
        .find_by_mod_id("mod-ghost", MC, LOADER)
        .await
        .unwrap();

    assert!(found.is_empty());
}

/// Modrinth not knowing the project passes the hand, it doesn't conclude.
#[tokio::test]
async fn an_empty_response_passes_to_the_next_source() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("registry-empty");
    server.code("/project/jei", 404);
    publish_web(&server, 42, "jei");

    let found = registry(&workshop, &server)
        .find_by_mod_id("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found[0].origin, crate::Origin::CurseForge);
}

/// A build pinned by the lockfile goes through the site, wrapper included.
#[tokio::test]
async fn a_pinned_build_goes_through_the_site() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("registry-pinned");
    server.json("/widget/jei", r#"{"id":42,"title":"JEI"}"#);
    server.json(
        "/web/mods/42/files/2",
        r#"{"data":{"id":2,"fileName":"jei-web.jar","displayName":"1.0","fileLength":1,
            "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
            "gameVersions":["1.21.1","NeoForge"]}}"#,
    );

    let found = registry(&workshop, &server)
        .curseforge_file("jei", "2")
        .await
        .unwrap()
        .expect("the build exists");

    assert_eq!(found.file_name, "jei-web.jar");
}

/// A pinned build that no longer exists gives nothing: it's up to the caller
/// to decide that's an error, with the mod's name in front of them.
#[tokio::test]
async fn a_pinned_build_that_disappeared_gives_nothing() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("registry-pinned-missing");
    server.json("/widget/jei", r#"{"id":42,"title":"JEI"}"#);
    server.code("/web/mods/42/files/9999", 404);

    assert!(
        registry(&workshop, &server)
            .curseforge_file("jei", "9999")
            .await
            .unwrap()
            .is_none()
    );
}
