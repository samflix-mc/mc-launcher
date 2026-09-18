use super::super::CurseForgeWeb;
use std::sync::Arc;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

/// A client whose two roots both point at the test server.
fn client(server: &mc_testkit::Server) -> CurseForgeWeb {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForgeWeb::with_bases(dl, &server.base(), &server.url("/widget"))
}

/// A file as the site's route renders it.
fn file(id: u64, name: &str, versions: &str) -> String {
    format!(
        r#"{{"id":{id},"fileName":"{name}","displayName":"{name}","fileLength":1024,
             "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
             "gameVersions":[{versions}]}}"#
    )
}

fn page(files: &[String], total: usize) -> String {
    format!(
        r#"{{"data":[{}],"pagination":{{"totalCount":{total}}}}}"#,
        files.join(",")
    )
}

#[tokio::test]
async fn a_slug_is_resolved_by_cfwidget_then_its_files_listed() {
    let server = mc_testkit::Server::new().await;
    server.json(
        "/widget/jei",
        r#"{"id":238222,"title":"Just Enough Items"}"#,
    );
    server.json(
        "/mods/238222/files",
        &page(&[file(5001, "jei-19.jar", r#""1.21.1","NeoForge""#)], 1),
    );
    server.json("/mods/238222/dependencies", r#"{"data":[]}"#);

    let found = client(&server).candidates("jei", MC, LOADER).await.unwrap();

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].name, "Just Enough Items");
    assert_eq!(found[0].project_id, "238222");
    // No digest published by this source: it will be computed on download
    // and then frozen into the lockfile.
    assert!(found[0].sha1.is_none());
    // The CDN URL isn't reconstructed: it's that reconstruction that would
    // bypass an author's refusal to be redistributed.
    assert!(found[0].url.ends_with("/mods/238222/files/5001/download"));
}

/// `gameVersions` mixes versions, loaders and sides. A Fabric file must not
/// be served to a NeoForge pack.
#[tokio::test]
async fn a_file_from_another_loader_is_discarded() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/jei", r#"{"id":1,"title":"JEI"}"#);
    server.json(
        "/mods/1/files",
        &page(
            &[
                file(1, "jei-fabric.jar", r#""1.21.1","Fabric""#),
                file(2, "jei-neo.jar", r#""1.21.1","NeoForge","Client""#),
            ],
            2,
        ),
    );
    server.json("/mods/1/dependencies", r#"{"data":[]}"#);

    let found = client(&server).candidates("jei", MC, LOADER).await.unwrap();

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].file_name, "jei-neo.jar");
}

/// Pagination is ignored by the server and `pageSize` is capped: a mod that
/// has published more than fifty files since its last compatible version
/// becomes invisible. Staying silent would be misleading.
#[tokio::test]
async fn a_version_outside_the_visible_window_is_reported() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/oldmod", r#"{"id":7,"title":"Old Mod"}"#);
    server.json(
        "/mods/7/files",
        &page(&[file(1, "old-1.20.jar", r#""1.20.1","NeoForge""#)], 400),
    );

    let error = client(&server)
        .candidates("oldmod", MC, LOADER)
        .await
        .expect_err("nothing compatible within the window");

    let text = format!("{error:#}");
    assert!(text.contains("400"), "{text}");
    // The way forward, not a setting that no longer exists.
    assert!(text.contains("file"), "{text}");
}

/// Nothing found and nothing more to see: that's an ordinary absence, not an
/// error — the caller must be able to carry on.
#[tokio::test]
async fn an_ordinary_absence_does_not_raise_an_error() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/absent", r#"{"id":9,"title":"Absent"}"#);
    server.json("/mods/9/files", &page(&[], 0));
    server.json("/mods/9/dependencies", r#"{"data":[]}"#);

    let found = client(&server)
        .candidates("absent", MC, LOADER)
        .await
        .unwrap();
    assert!(found.is_empty());
}

/// The site's dependencies are declared per project, not per file: they hold
/// for every version.
#[tokio::test]
async fn the_projects_dependencies_are_copied_onto_each_version() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/mod", r#"{"id":3,"title":"Mod"}"#);
    server.json(
        "/mods/3/files",
        &page(
            &[
                file(1, "a.jar", r#""1.21.1","NeoForge""#),
                file(2, "b.jar", r#""1.21.1","NeoForge""#),
            ],
            2,
        ),
    );
    server.json(
        "/mods/3/dependencies",
        r#"{"data":[{"id":42,"slug":"bookshelf","type":"RequiredDependency"},
                    {"id":43,"slug":"jei","type":"OptionalDependency"}]}"#,
    );

    let found = client(&server).candidates("mod", MC, LOADER).await.unwrap();

    assert_eq!(found.len(), 2);
    for candidate in &found {
        // Only the required ones count, and the slug is preferred over the
        // id: it makes it possible to find the project on Modrinth.
        assert_eq!(
            candidate.declared_deps.len(),
            1,
            "{:?}",
            candidate.declared_deps
        );
        assert_eq!(candidate.declared_deps[0].project_id, "bookshelf");
    }
}

/// A dependency without a slug leaves only the numeric id: better to track
/// that than to lose the dependency.
#[tokio::test]
async fn a_dependency_without_a_slug_falls_back_to_its_id() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/mod", r#"{"id":3,"title":"Mod"}"#);
    server.json(
        "/mods/3/files",
        &page(&[file(1, "a.jar", r#""1.21.1","NeoForge""#)], 1),
    );
    server.json(
        "/mods/3/dependencies",
        r#"{"data":[{"id":42,"slug":"","type":"RequiredDependency"}]}"#,
    );

    let found = client(&server).candidates("mod", MC, LOADER).await.unwrap();
    assert_eq!(found[0].declared_deps[0].project_id, "42");
}

/// A numeric id comes from an already-resolved dependency: the readable name
/// isn't essential, and skipping it saves a call to cfwidget, a volunteer
/// third-party service.
#[tokio::test]
async fn a_numeric_id_avoids_the_call_to_cfwidget() {
    let server = mc_testkit::Server::new().await;
    server.json(
        "/mods/238222/files",
        &page(&[file(1, "a.jar", r#""1.21.1","NeoForge""#)], 1),
    );
    server.json("/mods/238222/dependencies", r#"{"data":[]}"#);

    let found = client(&server)
        .candidates("238222", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(server.calls("/widget/238222"), 0);
}

/// 403 is Cloudflare's response just as much as a closed route: in both
/// cases this source has nothing to offer and the caller must carry on.
#[tokio::test]
async fn a_refusal_from_the_site_counts_as_absence_not_failure() {
    let server = mc_testkit::Server::new().await;
    server.json("/widget/jei", r#"{"id":1,"title":"JEI"}"#);
    server.code("/mods/1/files", 403);

    let found = client(&server).candidates("jei", MC, LOADER).await.unwrap();
    assert!(found.is_empty());
}

#[tokio::test]
async fn an_unknown_slug_from_cfwidget_yields_nothing() {
    let server = mc_testkit::Server::new().await;
    server.code("/widget/unknown", 404);

    let found = client(&server)
        .candidates("unknown", MC, LOADER)
        .await
        .unwrap();
    assert!(found.is_empty());
}
