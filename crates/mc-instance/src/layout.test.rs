use super::{Layout, PathBuf};

#[test]
fn instances_only_share_what_is_common() {
    let layout = Layout::new(PathBuf::from("/data"));
    let one = layout.instance("samflix");
    let other = layout.instance("test");

    assert_eq!(
        one.game_dir,
        PathBuf::from("/data/instances/samflix/minecraft")
    );
    assert_eq!(
        one.mods_dir(),
        PathBuf::from("/data/instances/samflix/minecraft/mods")
    );
    assert_ne!(one.mods_dir(), other.mods_dir());
    // Libraries and assets, though, are shared.
    assert_eq!(layout.shared(), PathBuf::from("/data/shared"));
}

/// The launcher's four directories all fit under a single root: one place
/// to delete to start from scratch.
#[test]
fn everything_fits_under_a_single_root() {
    let layout = Layout::new(PathBuf::from("/data"));

    assert_eq!(layout.runtime(), PathBuf::from("/data/runtime"));
    assert_eq!(layout.cache(), PathBuf::from("/data/cache"));
    for path in [
        layout.shared(),
        layout.runtime(),
        layout.cache(),
        layout.instance("samflix").dir,
    ] {
        assert!(path.starts_with("/data"), "{}", path.display());
    }
}

#[test]
fn the_default_layout_follows_the_data_directory() {
    assert_eq!(Layout::default().root, mc_paths::current().data);
}

#[test]
fn a_new_instance_gets_its_three_directories() {
    let tree = crate::fixtures::Tree::new("layout");
    let layout = Layout::new(tree.root.clone());
    let instance = layout.instance("samflix");

    instance.create().expect("the directories are created");

    assert!(instance.game_dir.is_dir());
    assert!(instance.mods_dir().is_dir());
    assert!(instance.config_dir().is_dir());
    assert_eq!(instance.name, "samflix");
}

/// A root that can't be created must say so, with the offending path: it's
/// a full disk or a read-only mount, and the message is all the reader has.
#[test]
fn an_impossible_directory_names_the_offending_path() {
    let tree = crate::fixtures::Tree::new("layout-blocked");
    let obstacle = tree.root.join("instances");
    std::fs::write(&obstacle, b"not a directory").unwrap();

    let error = Layout::new(tree.root.clone())
        .instance("samflix")
        .create()
        .expect_err("a file blocks the way");

    assert!(format!("{error:#}").contains("samflix"), "{error:#}");
}
