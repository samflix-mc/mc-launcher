use super::assemble;
use crate::fixtures::{NEOFORGE, Tree, VANILLA};
use crate::launch::descriptor::resolve_chain;
use crate::launch::session::LaunchOptions;
use crate::launch::variables::active_features;
use crate::vanilla::Features;
use std::collections::BTreeMap;

const OS: &str = "linux";
const ARCH: &str = "x86_64";

/// The chain read from a fake tree: `VersionJson` only builds through
/// deserialization, and that's just as well — the format read is the one
/// Mojang publishes.
fn chain(
    name: &str,
    descriptors: &[(&str, &str)],
) -> (Tree, Vec<crate::launch::descriptor::VersionJson>) {
    let tree = Tree::new(name);
    for (id, json) in descriptors {
        tree.version(id, json);
    }
    let chain = resolve_chain(&tree.shared(), descriptors[0].0).unwrap();
    (tree, chain)
}

fn variables() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("auth_player_name".into(), "Sam".into()),
        ("auth_uuid".into(), "0123".into()),
        ("classpath".into(), "/a.jar".into()),
        ("natives_directory".into(), "/natives".into()),
        ("library_directory".into(), "/libraries".into()),
        ("quickPlayMultiplayer".into(), "mc.ggy.info".into()),
    ])
}

/// The base lays down its arguments, the loader adds its own on top: the
/// chain is walked in the reverse order of inheritance.
#[test]
fn the_base_comes_before_the_loader() {
    let (_tree, chain) = chain(
        "asm-order",
        &[("neoforge-21.1.250", NEOFORGE), ("1.21.1", VANILLA)],
    );

    let args = assemble(
        &chain,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "cpw.mods.bootstraplauncher.BootstrapLauncher".into(),
        &LaunchOptions::default(),
    );

    let jvm_base = args
        .iter()
        .position(|a| a.starts_with("-Djava.library.path"));
    let jvm_loader = args
        .iter()
        .position(|a| a.starts_with("-DlibraryDirectory"));
    assert!(
        jvm_base < jvm_loader,
        "the loader comes before the base: {args:?}"
    );

    // The main class separates the JVM arguments from the game's.
    let main_class_position = args
        .iter()
        .position(|a| a == "cpw.mods.bootstraplauncher.BootstrapLauncher")
        .expect("the main class is present");
    assert!(jvm_loader.unwrap() < main_class_position);
    assert!(args.iter().position(|a| a == "--launchTarget").unwrap() > main_class_position);
}

/// A conditional argument only appears if its flag is active. Otherwise the
/// game refuses the command line.
#[test]
fn conditional_arguments_follow_the_flags() {
    let (_tree, chain) = chain("asm-flags", &[("1.21.1", VANILLA)]);
    let options = LaunchOptions {
        quick_play: Some(crate::launch::session::QuickPlay::Multiplayer(
            "mc.ggy.info".into(),
        )),
        ..Default::default()
    };

    let with = assemble(
        &chain,
        OS,
        ARCH,
        &active_features(&options),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "M".into(),
        &options,
    );
    assert!(with.contains(&"--quickPlayMultiplayer".into()), "{with:?}");
    assert!(with.contains(&"mc.ggy.info".into()), "{with:?}");
    // Resolution wasn't requested: its arguments stay absent.
    assert!(!with.contains(&"--width".into()), "{with:?}");

    let without = assemble(
        &chain,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "M".into(),
        &LaunchOptions::default(),
    );
    assert!(
        !without.contains(&"--quickPlayMultiplayer".into()),
        "{without:?}"
    );
}

/// Pre-2017 format: the descriptor doesn't describe JVM arguments, and the
/// game's fit on a single string.
#[test]
fn a_pre_2017_descriptor_gets_a_hand_placed_classpath() {
    let (_tree, chain) = chain(
        "asm-legacy",
        &[(
            "1.21.1",
            r#"{"id":"1.21.1","mainClass":"M","assets":"legacy",
                "minecraftArguments":"--username ${auth_player_name} --uuid ${auth_uuid}"}"#,
        )],
    );

    let args = assemble(
        &chain,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar:/b.jar",
        "M".into(),
        &LaunchOptions::default(),
    );

    assert_eq!(args[0], "-Djava.library.path=/natives");
    assert_eq!(args[1], "-cp");
    assert_eq!(args[2], "/a.jar:/b.jar");
    assert!(args.contains(&"Sam".into()), "{args:?}");
    assert!(args.contains(&"0123".into()), "{args:?}");
}

/// Memory comes before the descriptor's arguments, so that an explicit
/// setting can still be overridden by whatever the loader insists on.
#[test]
fn memory_and_extra_flags_open_the_line() {
    let (_tree, chain) = chain("asm-memory", &[("1.21.1", VANILLA)]);

    let args = assemble(
        &chain,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "M".into(),
        &LaunchOptions {
            memory_mb: Some(6144),
            extra_jvm: vec!["-XX:+UseZGC".into()],
            ..Default::default()
        },
    );

    assert_eq!(args[0], "-Xmx6144M");
    assert_eq!(args[1], "-XX:+UseZGC");
}
