use super::classpath;
use crate::fixtures::{NEOFORGE, Tree, VANILLA};
use crate::launch::descriptor::resolve_chain;

const OS: &str = "linux";
const ARCH: &str = "x86_64";

/// NeoForge replaces some of Mojang's libraries. Its version has to come
/// first, otherwise the JVM loads Mojang's and the loader fails on a
/// missing method.
#[test]
fn the_loaders_library_replaces_the_games() {
    let tree = Tree::new("cp-replacement");
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        .library("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .library("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");

    let chain = resolve_chain(&tree.shared(), "neoforge-21.1.250").unwrap();
    let (paths, text) = classpath(&chain, &tree.shared(), OS, ARCH, ":").unwrap();

    let names: Vec<String> = paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(
        names.contains(&"guava-33.0.0-jre.jar".to_string()),
        "{names:?}"
    );
    assert!(
        !names.contains(&"guava-32.1.2-jre.jar".to_string()),
        "both guavas are on the classpath: {names:?}"
    );
    assert_eq!(text.split(':').count(), paths.len());
}

/// Under a loader, the installer has produced its own split of the client.
/// Adding `1.21.1.jar` on top yields two modules exporting the same
/// packages, and the JVM stops before the first screen.
#[test]
fn the_vanilla_client_stays_off_the_classpath_under_a_loader() {
    let tree = Tree::new("cp-loaded-client");
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        .library("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .library("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");

    let chain = resolve_chain(&tree.shared(), "neoforge-21.1.250").unwrap();
    let (paths, _) = classpath(&chain, &tree.shared(), OS, ARCH, ":").unwrap();

    assert!(
        !paths.iter().any(|p| p.ends_with("1.21.1.jar")),
        "the vanilla client is on the classpath: {paths:?}"
    );
}

/// In pure vanilla, on the other hand, it's the one carrying the game.
#[test]
fn the_vanilla_client_joins_the_classpath_without_a_loader() {
    let tree = Tree::new("cp-bare-client");
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .library("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    let (paths, _) = classpath(&chain, &tree.shared(), OS, ARCH, ":").unwrap();

    assert!(paths.iter().any(|p| p.ends_with("1.21.1.jar")), "{paths:?}");
}

/// A library reserved for another system doesn't need to be downloaded or
/// required: two thirds of the vanilla classpath carry no rule, the rest
/// depends on the system.
#[test]
fn a_library_for_another_system_is_ignored() {
    let tree = Tree::new("cp-other-system");
    tree.version("1.21.1", VANILLA)
        .client("1.21.1")
        .library("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    let (paths, _) = classpath(&chain, &tree.shared(), OS, ARCH, ":").unwrap();

    assert!(
        !paths.iter().any(|p| p.to_string_lossy().contains("lwjgl")),
        "a macOS library was retained under linux: {paths:?}"
    );
}

/// A missing library fails the startup anyway; saying so here, with the
/// name of the first missing file, spares a Java stack trace.
#[test]
fn a_missing_library_is_named() {
    let tree = Tree::new("cp-missing");
    tree.version("1.21.1", VANILLA).client("1.21.1");

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    let error = classpath(&chain, &tree.shared(), OS, ARCH, ":").expect_err("guava is missing");

    let text = format!("{error:#}");
    assert!(text.contains("guava"), "{text}");
    assert!(text.contains("reinstall"), "{text}");
}

#[test]
fn a_missing_client_is_reported_as_such() {
    let tree = Tree::new("cp-no-client");
    tree.version("1.21.1", VANILLA)
        .library("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    let error = classpath(&chain, &tree.shared(), OS, ARCH, ":").expect_err("no client");

    assert!(format!("{error:#}").contains("missing client"), "{error:#}");
}

/// A library with no `downloads` falls back to the Maven path derived from
/// its name — the norm for the ones NeoForge adds.
#[test]
fn a_library_without_a_download_falls_back_to_its_maven_path() {
    let tree = Tree::new("cp-maven");
    tree.version(
        "1.21.1",
        r#"{"id":"1.21.1","mainClass":"M","assetIndex":{"id":"17"},
            "libraries":[{"name":"net.neoforged:mergetool:2.0.0"}]}"#,
    )
    .client("1.21.1")
    .library("net/neoforged/mergetool/2.0.0/mergetool-2.0.0.jar");

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    let (paths, _) = classpath(&chain, &tree.shared(), OS, ARCH, ":").unwrap();

    assert!(
        paths.iter().any(|p| p.ends_with("mergetool-2.0.0.jar")),
        "{paths:?}"
    );
}

/// A name that doesn't look like anything yields no path: better to stop
/// than to silently assemble an incomplete classpath.
#[test]
fn an_unusable_library_name_stops_the_assembly() {
    let tree = Tree::new("cp-broken-name");
    tree.version(
        "1.21.1",
        r#"{"id":"1.21.1","mainClass":"M","assetIndex":{"id":"17"},
            "libraries":[{"name":"notwocolons"}]}"#,
    )
    .client("1.21.1");

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    let error = classpath(&chain, &tree.shared(), OS, ARCH, ":").expect_err("unusable name");

    assert!(format!("{error:#}").contains("no usable path"), "{error:#}");
}
