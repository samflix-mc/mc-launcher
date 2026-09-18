use super::{Path, parse_major, probe};
use crate::locations::{candidates, managed_home};

#[test]
fn major_of_both_version_schemes() {
    assert_eq!(parse_major("21.0.5+11"), Some(21));
    assert_eq!(parse_major("21"), Some(21));
    assert_eq!(parse_major("17.0.9"), Some(17));
    // Up to Java 8, the major is the second number.
    assert_eq!(parse_major("1.8.0_412"), Some(8));
    assert_eq!(parse_major("1.7.0_80"), Some(7));
    assert_eq!(parse_major("22-ea"), Some(22));
    assert_eq!(parse_major(""), None);
}

/// A lone "1" says nothing: the second number is missing, and guessing would
/// pass an unknown runtime off as a Java 1.
#[test]
fn a_truncated_scheme_yields_no_major() {
    assert_eq!(parse_major("1"), None);
    assert_eq!(parse_major("1.x"), None);
    assert_eq!(parse_major("nothing at all"), None);
}

#[test]
fn the_managed_runtime_is_the_first_candidate() {
    let dir = Path::new("/tmp/mc-runtime");
    let list = candidates(dir, 21);
    assert_eq!(list[0], managed_home(dir, 21).join("bin").join("java"));
}

/// `-version` writes to stderr — a historical JVM choice — and across three
/// lines, only the first of which carries the number, in quotes. Reading
/// standard output would give nothing.
#[cfg(unix)]
#[tokio::test]
async fn the_version_is_read_on_standard_error() {
    let _workshop = crate::fixtures::workshop();
    let tree = crate::fixtures::Tree::new("probe");
    let exe = tree.root.join("bin").join("java");
    crate::fixtures::fake_java(&exe, "21.0.5+11");

    let version = probe(&exe).await.expect("the binary responds");

    assert_eq!(version.major, 21);
    assert_eq!(version.full, "21.0.5+11");
}

#[cfg(unix)]
#[tokio::test]
async fn a_java_8_is_recognized_as_such() {
    let _workshop = crate::fixtures::workshop();
    let tree = crate::fixtures::Tree::new("probe-8");
    let exe = tree.root.join("bin").join("java");
    crate::fixtures::fake_java(&exe, "1.8.0_412");

    let version = probe(&exe).await.unwrap();
    assert_eq!(version.major, 8, "a Java 8 was passed off as a Java 1");
}

/// A binary present but silent — half-uninstalled package — must not be
/// retained on the strength of its path alone.
#[cfg(unix)]
#[tokio::test]
async fn a_binary_that_says_nothing_is_refused() {
    let _workshop = crate::fixtures::workshop();
    let tree = crate::fixtures::Tree::new("probe-silent");
    let exe = tree.root.join("bin").join("java");
    crate::fixtures::silent_java(&exe);

    let error = probe(&exe).await.expect_err("nothing usable");
    assert!(format!("{error:#}").contains("unreadable"), "{error:#}");
}

#[tokio::test]
async fn a_missing_binary_is_reported_with_its_path() {
    let error = probe(Path::new("/usr/lib/jvm/absent/bin/java"))
        .await
        .expect_err("nothing at this location");
    assert!(format!("{error:#}").contains("absent"), "{error:#}");
}
