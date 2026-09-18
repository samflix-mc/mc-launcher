use crate::fixtures::{Project, Version, jar, publish};
use crate::modrinth::Modrinth;
use std::sync::Arc;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn client(server: &mc_testkit::Server) -> Modrinth {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    Modrinth::with_base(dl, &server.base())
}

/// Search by `modId` walks the results in relevance order and stops at the
/// first project that actually has a compatible version.
///
/// Stopping at the first result, compatible or not, would yield an empty
/// list as soon as a namesake project — abandoned, or published for another
/// game version — ranks first. The dependency would be declared missing
/// even though it's right there in second place, and the pack would install
/// without it.
#[tokio::test]
async fn search_by_mod_id_moves_to_the_next_result() {
    let server = mc_testkit::Server::new().await;

    let content = jar("bookshelf", &[]);
    server.bytes("/bookshelf.jar", &content);
    // The second project has a version; the first has none.
    publish(&server, &Project::new("abandoned"));
    publish(
        &server,
        &Project::new("bookshelf").version(Version::new(
            "20.2.0",
            &server.url("/bookshelf.jar"),
            &content,
        )),
    );
    server.json(
        "/search",
        r#"{"hits":[{"slug":"abandoned","project_id":"abandoned-id","title":"Abandoned"},
                    {"slug":"bookshelf","project_id":"bookshelf-id","title":"Bookshelf"}]}"#,
    );

    // The modId isn't the project's slug — that's the case that forces the
    // search path, and therefore walking its results.
    let found = client(&server)
        .find_by_mod_id("bookshelflib", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].slug, "bookshelf");
}

/// No result isn't an error: a `modId` read from a jar may match no
/// published project — a bundled library, a removed mod. The caller has
/// other sources to check.
#[tokio::test]
async fn a_search_with_no_result_does_not_raise_an_error() {
    let server = mc_testkit::Server::new().await;
    server.json("/search", r#"{"hits":[]}"#);

    let found = client(&server)
        .find_by_mod_id("ghost-mod", MC, LOADER)
        .await
        .expect("finding nothing isn't a failure");
    assert!(found.is_empty());
}
