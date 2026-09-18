use super::{extract, single_child};
use crate::fixtures::{Tree, temurin_archive};

/// The archive contains a root folder named after the version, which we
/// don't want in the final path: it's that folder `single_child` finds.
#[cfg(unix)]
#[test]
fn a_tar_gz_archive_unpacks_into_a_single_directory() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("tar");
    let archive = tree.root.join("temurin.tar.gz");
    std::fs::write(&archive, temurin_archive("21.0.5+11")).unwrap();
    let target = tree.root.join("extraction");
    std::fs::create_dir_all(&target).unwrap();

    extract(&archive, &target).expect("the tar.gz unpacks");

    let root = single_child(&target).expect("a single root directory");
    assert!(root.join("bin").join("java").is_file());
}

#[test]
fn two_entries_at_the_root_is_an_unexpected_archive() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("two-entries");
    std::fs::write(tree.root.join("one"), b"").unwrap();
    std::fs::write(tree.root.join("two"), b"").unwrap();

    let error = single_child(&tree.root).expect_err("two entries");
    assert!(format!("{error:#}").contains("2 entries"), "{error:#}");
}

#[test]
fn an_empty_archive_is_refused_too() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("empty");
    let error = single_child(&tree.root).expect_err("no entry");
    assert!(format!("{error:#}").contains("0 entries"), "{error:#}");
}

/// Adoptium publishes `.tar.gz` and `.zip`; any other suffix signals a change
/// on their end, which is better seen right away.
#[test]
fn an_unknown_format_is_refused_by_its_name() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("format");
    let archive = tree.root.join("temurin.7z");
    std::fs::write(&archive, b"").unwrap();

    let error = extract(&archive, &tree.root).expect_err("unsupported format");
    assert!(format!("{error:#}").contains("temurin.7z"), "{error:#}");
}

#[test]
fn an_unreadable_tar_gz_is_reported_with_its_name() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("tar-broken");
    let archive = tree.root.join("temurin.tar.gz");
    std::fs::write(&archive, b"this is not gzip").unwrap();

    let error = extract(&archive, &tree.root).expect_err("invalid gzip");
    assert!(format!("{error:#}").contains("temurin.tar.gz"), "{error:#}");
}

/// A ZIP archive can contain upward paths that would write outside the
/// target directory. A JDK is code run with the user's privileges: the entry
/// must be refused, not written elsewhere.
#[test]
fn a_valid_zip_unpacks_with_its_permissions() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("zip");
    let archive = tree.root.join("temurin.zip");

    let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    writer.add_directory("jdk-21/bin/", options).unwrap();
    writer.start_file("jdk-21/bin/java", options).unwrap();
    {
        use std::io::Write;
        writer.write_all(b"#!/bin/sh\n").unwrap();
    }
    writer.finish().unwrap();

    let target = tree.root.join("extraction");
    extract(&archive, &target).expect("the zip unpacks");

    let placed = target.join("jdk-21").join("bin").join("java");
    assert!(placed.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&placed).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "the binary is not executable");
    }
}
