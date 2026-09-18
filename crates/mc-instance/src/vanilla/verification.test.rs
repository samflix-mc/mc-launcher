use super::{classpath, verify_assets};
use crate::fixtures::{NEOFORGE, Tree, VANILLA};

fn descriptor(tree: &Tree, id: &str) -> std::path::PathBuf {
    tree.shared()
        .join("versions")
        .join(id)
        .join(format!("{id}.json"))
}

/// The descriptor NeoForge produces carries neither `assetIndex` nor
/// `downloads`: reading it with the full structure would fail, even though
/// its libraries matter just as much as Mojang's.
#[test]
fn a_loader_descriptor_reads_like_mojang_s() {
    let tree = Tree::new("verify-loader");
    tree.version("neoforge-21.1.250", NEOFORGE);

    let libs = classpath(&descriptor(&tree, "neoforge-21.1.250"), &tree.shared()).unwrap();

    assert_eq!(libs.len(), 2, "{libs:?}");
    assert!(
        libs.iter().any(|p| p.ends_with("guava-33.0.0-jre.jar")),
        "{libs:?}"
    );
    // Paths are relative to the shared store.
    assert!(
        libs.iter()
            .all(|p| p.starts_with(tree.shared().join("libraries")))
    );
}

#[test]
fn a_library_reserved_for_another_system_is_not_required() {
    let tree = Tree::new("verify-system");
    tree.version("1.21.1", VANILLA);

    let libs = classpath(&descriptor(&tree, "1.21.1"), &tree.shared()).unwrap();

    // On Linux, LWJGL's macOS native doesn't need to be present.
    assert!(
        !libs.iter().any(|p| p.to_string_lossy().contains("lwjgl")),
        "{libs:?}"
    );
}

#[test]
fn an_unreadable_descriptor_is_reported_with_its_path() {
    let tree = Tree::new("verify-broken");
    tree.version("1.21.1", "not JSON");

    let error = classpath(&descriptor(&tree, "1.21.1"), &tree.shared()).expect_err("broken");
    assert!(format!("{error:#}").contains("unreadable"), "{error:#}");
}

/// Complement to the quick check, which only compares sizes: here every
/// object is reread and its digest recomputed.
#[test]
fn an_intact_asset_is_counted_as_such() {
    let tree = Tree::new("assets-intact");
    let digests = vec![tree.asset(b"one"), tree.asset(b"two")];
    tree.index_assets("17", &digests);

    let report = verify_assets(&tree.shared(), "17").unwrap();

    assert_eq!(report.ok, 2);
    assert!(report.is_clean());
}

#[test]
fn a_missing_asset_is_named_by_its_digest() {
    let tree = Tree::new("assets-missing");
    let present = tree.asset(b"one");
    let missing = "0000000000000000000000000000000000000000".to_string();
    tree.index_assets("17", &[present, missing.clone()]);

    let report = verify_assets(&tree.shared(), "17").unwrap();

    assert_eq!(report.ok, 1);
    assert_eq!(report.missing, vec![missing]);
    assert!(!report.is_clean());
}

/// An object whose content no longer matches its name: exactly what the
/// quick check lets through, and what this one must catch.
#[test]
fn a_corrupt_asset_is_distinguished_from_a_missing_one() {
    let tree = Tree::new("assets-corrupt");
    let digest = tree.asset(b"one");
    let path = tree
        .shared()
        .join("assets")
        .join("objects")
        .join(&digest[..2])
        .join(&digest);
    std::fs::write(&path, b"something else").unwrap();
    tree.index_assets("17", std::slice::from_ref(&digest));

    let report = verify_assets(&tree.shared(), "17").unwrap();

    assert_eq!(report.ok, 0);
    assert!(report.missing.is_empty());
    assert_eq!(report.corrupt, vec![digest]);
}

#[test]
fn a_missing_index_is_an_error_not_an_empty_report() {
    let tree = Tree::new("assets-no-index");
    assert!(verify_assets(&tree.shared(), "17").is_err());
}
