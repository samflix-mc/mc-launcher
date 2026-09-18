use super::build;
use crate::fixtures::{NEOFORGE, Tree, VANILLA};
use crate::launch::session::{LaunchOptions, QuickPlay, Session};

/// A complete install, loader included, ready to be launched.
fn installation(name: &str) -> Tree {
    let tree = Tree::new(name);
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        // The loader's wins over the game's: it's the only one of the two
        // that has to be present.
        .library("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .library("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");
    tree
}

fn session() -> Session {
    Session::offline("Sam", "0123456789abcdef0123456789abcdef")
}

#[test]
fn the_command_line_is_composed_of_the_base_and_the_loader() {
    let tree = installation("build");
    let command = build(
        "neoforge-21.1.250",
        &tree.shared(),
        &tree.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect("the command line assembles");

    // The main class is the loader's, not the game's: it's the first in the
    // chain to declare one that wins.
    assert!(
        command
            .args
            .contains(&"cpw.mods.bootstraplauncher.BootstrapLauncher".into()),
        "{:?}",
        command.args
    );
    // The assets index comes from the base, which the loader doesn't declare.
    assert!(command.args.contains(&"--launchTarget".into()));
    assert_eq!(command.java, std::path::Path::new("/usr/bin/java"));
    assert_eq!(command.working_dir, tree.game_dir());

    // The base's arguments come before the loader's, and the `${…}` are
    // resolved.
    assert!(command.args.contains(&"Sam".into()));
    assert!(!command.args.iter().any(|a| a.contains("${")));
}

/// LWJGL extracts its own binaries from the classpath jars, but refuses to
/// start if the directory doesn't exist. The game directory, meanwhile, is
/// where Minecraft will write `saves` and `options.txt`.
#[test]
fn the_directories_the_jvm_requires_are_created() {
    let tree = installation("directories");
    build(
        "neoforge-21.1.250",
        &tree.shared(),
        &tree.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .unwrap();

    assert!(tree.shared().join("natives").join("1.21.1").is_dir());
    assert!(tree.game_dir().is_dir());
}

/// `--quickPlayMultiplayer` only exists in the descriptor behind a rule
/// conditioned on the flag. Ignoring flag rules produces a command line the
/// game refuses.
#[test]
fn joining_a_server_adds_the_arguments_that_go_with_it() {
    let tree = installation("quickplay");
    let command = build(
        "neoforge-21.1.250",
        &tree.shared(),
        &tree.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions {
            quick_play: Some(QuickPlay::Multiplayer("mc.ggy.info".into())),
            memory_mb: Some(4096),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(command.args.contains(&"--quickPlayMultiplayer".into()));
    assert!(command.args.contains(&"mc.ggy.info".into()));
    // Memory comes before the descriptor's arguments, so that an explicit
    // setting can still be overridden by whatever the loader insists on.
    assert_eq!(command.args.first().map(String::as_str), Some("-Xmx4096M"));
    // Without a resolution request, its arguments stay absent.
    assert!(!command.args.contains(&"--width".into()));
}

#[test]
fn a_missing_version_is_reported_instead_of_panicking() {
    let tree = Tree::new("missing");
    let error = build(
        "neoforge-21.1.250",
        &tree.shared(),
        &tree.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect_err("nothing is installed");

    assert!(
        format!("{error:#}").contains("is not installed"),
        "{error:#}"
    );
}

/// A descriptor without a main class can't launch anything, and saying so
/// beats letting the JVM answer in our place.
#[test]
fn a_chain_without_a_main_class_is_rejected() {
    let tree = Tree::new("no-class");
    tree.version("1.21.1", r#"{"id":"1.21.1","assetIndex":{"id":"17"}}"#)
        .client("1.21.1");

    let error = build(
        "1.21.1",
        &tree.shared(),
        &tree.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect_err("no main class");

    assert!(format!("{error:#}").contains("main class"), "{error:#}");
}

/// Very old versions name their assets index `assets` rather than
/// `assetIndex`; both must be readable.
#[test]
fn an_old_style_assets_index_is_accepted() {
    let tree = Tree::new("old-assets");
    tree.version(
        "1.21.1",
        r#"{"id":"1.21.1","mainClass":"M","assets":"legacy",
            "minecraftArguments":"--username ${auth_player_name}"}"#,
    )
    .client("1.21.1");

    let command = build(
        "1.21.1",
        &tree.shared(),
        &tree.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect("a pre-2017 descriptor is still launchable");

    // With no JVM arguments declared, the classpath is placed by hand.
    assert!(command.args.contains(&"-cp".into()));
    assert!(command.args.contains(&"Sam".into()));
}
