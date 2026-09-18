use super::open;
use crate::fixtures::{MANIFEST, Workshop, entry};

/// Opens what's installed, not what's published: fetching today's pack would
/// describe mods the folder doesn't contain, and would forbid playing
/// without a network.
#[test]
fn the_opened_instance_is_the_one_thats_installed() {
    let workshop = Workshop::new("open-ok");
    let source = workshop.installed_pack(vec![entry("jei", "both", None)]);

    let (manifest, lock, instance) =
        open(&source, &workshop.options()).expect("everything is there");

    assert_eq!(manifest.name, "samflix");
    assert_eq!(lock.mods.len(), 1);
    assert_eq!(instance.name, "samflix");
    assert!(instance.mods_dir().join("jei.jar").is_file());
}

/// Without a lock, there's no game session to launch, and the message must
/// say what to do.
#[test]
fn without_a_lock_we_point_back_to_installation() {
    let workshop = Workshop::new("open-no-lock");
    let manifest = workshop.root.join("samflix.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    let options = workshop.options();
    let source = crate::source::Source::parse(manifest.to_str().unwrap(), &options.layout);

    let error = open(&source, &options).expect_err("no lock");
    assert!(
        format!("{error:#}").contains("mc-pack install"),
        "{error:#}"
    );
}

/// Absent `--instance`, the pack's name is used.
#[test]
fn absent_a_name_the_instance_carries_the_packs() {
    let workshop = Workshop::new("open-name");
    let source = workshop.installed_pack(vec![entry("jei", "both", None)]);
    let mut options = workshop.options();
    options.instance_name = None;

    let (_, _, instance) = open(&source, &options).expect("everything is there");
    assert_eq!(instance.name, "samflix");
}

/// An instance that doesn't match the lock is refused before launching:
/// starting anyway means letting the server settle it with an ejection that
/// doesn't name its cause.
#[test]
fn a_divergent_instance_is_refused_before_launch() {
    let workshop = Workshop::new("open-divergent");
    let source = workshop.installed_pack(vec![entry("jei", "both", None)]);
    let instance = workshop.options().layout.instance("samflix");
    std::fs::remove_file(instance.mods_dir().join("jei.jar")).unwrap();

    let error = open(&source, &workshop.options()).expect_err("the instance diverges");
    assert!(format!("{error:#}").contains("jei.jar"), "{error:#}");
}
