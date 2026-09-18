use super::install_from;
use crate::adoptium::platform;
use crate::fixtures::{Tree, adoptium_response, sha256, temurin_archive};
use crate::version::Origin;

/// The API path as `url_assets` composes it, to place the server's response
/// at the right spot.
fn path(major: u32, image: &str) -> String {
    let (_os, _arch) = platform().unwrap();
    format!("/assets/latest/{major}/hotspot?image_type={image}")
        .split('?')
        .next()
        .unwrap()
        .to_string()
}

#[cfg(unix)]
#[tokio::test]
async fn a_temurin_is_downloaded_unpacked_and_verified() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install");
    let archive = temurin_archive("21.0.5+11");

    server.bytes("/temurin.tar.gz", &archive);
    server.json(
        &path(21, "jre"),
        &adoptium_response(
            "jre",
            &server.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&archive),
        ),
    );

    let java = install_from(&server.base(), 21, &tree.root, None)
        .await
        .expect("the installation succeeds");

    assert_eq!(java.origin, Origin::Managed);
    assert_eq!(java.version.major, 21);
    assert_eq!(
        java.path,
        tree.root.join("temurin-21").join("bin").join("java")
    );
    // The archive and the extraction directory must leave nothing behind.
    assert!(!tree.root.join("temurin.tar.gz").exists());
    assert!(!tree.root.join(".temurin-21-extraction").exists());
}

/// Adoptium doesn't publish a JRE for every combination of platforms, hence
/// the fallback to the JDK — twice as heavy, but present everywhere.
#[cfg(unix)]
#[tokio::test]
async fn the_jdk_is_used_when_no_jre_is_available() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install-jdk");
    let archive = temurin_archive("21.0.5+11");

    server.bytes("/temurin.tar.gz", &archive);
    // The JRE exists in the response but under a different image type: this
    // is the real case where Adoptium returns something other than what was
    // asked for.
    server.json(&path(21, "jre"), "[]");
    server.json(
        &path(21, "jdk"),
        &adoptium_response(
            "jdk",
            &server.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&archive),
        ),
    );

    let java = install_from(&server.base(), 21, &tree.root, None)
        .await
        .expect("the JDK takes over");
    assert_eq!(java.version.major, 21);
}

/// A JDK is code run with the user's privileges: an archive whose digest
/// doesn't match must never be unpacked.
#[cfg(unix)]
#[tokio::test]
async fn an_archive_with_a_wrong_digest_is_not_installed() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install-digest");

    server.bytes("/temurin.tar.gz", b"<html>error page</html>");
    server.json(
        &path(21, "jre"),
        &adoptium_response(
            "jre",
            &server.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&temurin_archive("21.0.5+11")),
        ),
    );

    let error = install_from(&server.base(), 21, &tree.root, None)
        .await
        .expect_err("the digest doesn't match");

    assert!(format!("{error:#}").contains("SHA-256"), "{error:#}");
    assert!(!tree.root.join("temurin-21").exists());
}

#[tokio::test]
async fn a_java_nobody_publishes_is_reported_clearly() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install-missing");
    server.json(&path(99, "jre"), "[]");
    server.json(&path(99, "jdk"), "[]");

    let error = install_from(&server.base(), 99, &tree.root, None)
        .await
        .expect_err("nothing published");

    assert!(
        format!("{error:#}").contains("doesn't publish Java 99"),
        "{error:#}"
    );
}

#[tokio::test]
async fn an_unreadable_response_names_the_requested_image_type() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install-unreadable");
    server.json(&path(21, "jre"), "this is not JSON");

    let error = install_from(&server.base(), 21, &tree.root, None)
        .await
        .expect_err("broken response");

    assert!(format!("{error:#}").contains("jre 21"), "{error:#}");
}

/// Adoptium sometimes returns something other than what was asked for. The
/// entry is then discarded, and the search continues on the next image:
/// keeping it would install a package whose contents are unknown, under a
/// name that claims otherwise.
#[cfg(unix)]
#[tokio::test]
async fn an_image_of_a_different_type_than_requested_is_discarded() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install-wrong-type");

    // A single response, two entries. The first doesn't carry the requested
    // type and would deliver a Java 17; it's the second that must be
    // retained. (The test server ignores the query string: both images share
    // the same path, which is precisely the situation where only
    // `image_type` can tell them apart.)
    let decoy = temurin_archive("17.0.9");
    server.bytes("/decoy.tar.gz", &decoy);
    let expected = temurin_archive("21.0.5+11");
    server.bytes("/temurin.tar.gz", &expected);

    let two_entries = format!(
        "[{},{}]",
        one_entry(
            "jdk",
            &server.url("/decoy.tar.gz"),
            "decoy.tar.gz",
            &sha256(&decoy)
        ),
        one_entry(
            "jre",
            &server.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&expected)
        ),
    );
    server.json(&path(21, "jre"), &two_entries);

    let java = install_from(&server.base(), 21, &tree.root, None)
        .await
        .expect("the entry of the right type is retained");
    assert_eq!(java.version.major, 21);
}

/// One entry of the Adoptium response, without the brackets: to compose
/// several of them into a single response.
fn one_entry(image: &str, link: &str, name: &str, sha256: &str) -> String {
    let single = adoptium_response(image, link, name, sha256);
    single
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

/// The last check, and the only one that proves anything: the installed
/// binary starts and reports the required version. Adoptium can publish
/// under one name what it delivers under another, and a Java too old stops
/// the game on `UnsupportedClassVersionError` before even showing a window.
#[cfg(unix)]
#[tokio::test]
async fn a_temurin_reporting_a_version_too_low_is_refused() {
    let server = mc_testkit::Server::new().await;
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("install-too-old");

    let archive = temurin_archive("17.0.9");
    server.bytes("/temurin.tar.gz", &archive);
    server.json(&path(21, "jre"), "[]");
    server.json(
        &path(21, "jdk"),
        &adoptium_response(
            "jdk",
            &server.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&archive),
        ),
    );

    let error = install_from(&server.base(), 21, &tree.root, None)
        .await
        .expect_err("a Java 17 does not answer a request for Java 21");
    let text = format!("{error:#}");
    assert!(text.contains("17"), "{text}");
    assert!(text.contains("21"), "{text}");
}
