use super::{Layout, PathBuf};

#[test]
fn les_instances_ne_partagent_que_le_commun() {
    let layout = Layout::new(PathBuf::from("/data"));
    let une = layout.instance("samflix");
    let autre = layout.instance("essai");

    assert_eq!(
        une.game_dir,
        PathBuf::from("/data/instances/samflix/minecraft")
    );
    assert_eq!(
        une.mods_dir(),
        PathBuf::from("/data/instances/samflix/minecraft/mods")
    );
    assert_ne!(une.mods_dir(), autre.mods_dir());
    // Bibliothèques et assets, eux, sont communs.
    assert_eq!(layout.shared(), PathBuf::from("/data/shared"));
}
