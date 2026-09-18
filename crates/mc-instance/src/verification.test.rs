use super::verify;
use crate::fixtures::{NEOFORGE, Tree, VANILLA};
use crate::layout::Layout;

/// `Layout` puts `shared` under its root; the fixture tree follows the same
/// layout, so its root makes a valid `Layout`.
fn layout(tree: &Tree) -> Layout {
    Layout::new(tree.root.clone())
}

fn complete(name: &str) -> Tree {
    let tree = Tree::new(name);
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        .library("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar")
        .library("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .library("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");
    tree
}

#[test]
fn a_complete_installation_reports_nothing() {
    let tree = complete("verify-complete");
    let problems = verify("1.21.1", "21.1.250", &layout(&tree), false).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_missing_client_is_reported_by_its_path() {
    let tree = complete("verify-no-client");
    std::fs::remove_file(
        tree.shared()
            .join("versions")
            .join("1.21.1")
            .join("1.21.1.jar"),
    )
    .unwrap();

    let problems = verify("1.21.1", "21.1.250", &layout(&tree), false).unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("missing file"), "{problems:?}");
    assert!(problems[0].contains("1.21.1.jar"), "{problems:?}");
}

/// Both descriptors are checked: NeoForge's adds about fifty libraries, and
/// missing just one fails the startup as surely as a missing vanilla
/// library would.
#[test]
fn a_missing_loader_library_is_seen() {
    let tree = complete("verify-loader-lib");
    std::fs::remove_file(
        tree.shared()
            .join("libraries")
            .join("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar"),
    )
    .unwrap();

    let problems = verify("1.21.1", "21.1.250", &layout(&tree), false).unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("missing library"), "{problems:?}");
    assert!(problems[0].contains("loader-4.0.24.jar"), "{problems:?}");
}

#[test]
fn an_uninstalled_loader_is_named_with_its_version() {
    let tree = Tree::new("verify-no-loader");
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .library("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let problems = verify("1.21.1", "21.1.250", &layout(&tree), false).unwrap();
    assert!(
        problems.iter().any(|p| p.contains("NeoForge 21.1.250")),
        "{problems:?}"
    );
}

/// `deep` re-checks the digest of every asset object, which the installer
/// skips so it doesn't re-read 800 MB on every launch.
#[test]
fn the_deep_verification_rereads_the_assets() {
    let tree = complete("verify-deep");
    let digest = tree.asset(b"one");
    let path = tree
        .shared()
        .join("assets")
        .join("objects")
        .join(&digest[..2])
        .join(&digest);
    std::fs::write(&path, b"something else").unwrap();
    tree.index_assets("17", std::slice::from_ref(&digest));

    let without = verify("1.21.1", "21.1.250", &layout(&tree), false).unwrap();
    assert!(without.is_empty(), "{without:?}");

    let with = verify("1.21.1", "21.1.250", &layout(&tree), true).unwrap();
    assert!(with.iter().any(|p| p.contains("corrupt asset")), "{with:?}");
}

/// When files are already missing, re-reading 800 MB of assets wouldn't
/// teach anything more: it's the installation that needs re-running.
#[test]
fn the_deep_verification_does_not_add_to_problems_already_found() {
    let tree = complete("verify-deep-unneeded");
    std::fs::remove_file(
        tree.shared()
            .join("versions")
            .join("1.21.1")
            .join("1.21.1.jar"),
    )
    .unwrap();

    let problems = verify("1.21.1", "21.1.250", &layout(&tree), true).unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
}

/// Without an asset index on disk, the deep verification has nothing to
/// reread — and that's not an error, just a young installation.
#[test]
fn a_deep_verification_without_an_index_reports_nothing() {
    let tree = complete("verify-deep-empty");
    let problems = verify("1.21.1", "21.1.250", &layout(&tree), true).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
}
