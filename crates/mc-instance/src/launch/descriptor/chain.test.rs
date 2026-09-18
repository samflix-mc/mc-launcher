use super::{library_key, resolve_chain};
use crate::fixtures::{NEOFORGE, Tree, VANILLA};

#[test]
fn the_library_key_ignores_the_version() {
    // This is what makes it possible to see that one library replaces another.
    assert_eq!(
        library_key("com.google.guava:guava:32.1.2-jre"),
        "com.google.guava:guava"
    );
    assert_eq!(
        library_key("com.google.guava:guava:31.0-jre"),
        library_key("com.google.guava:guava:32.1.2-jre")
    );
}

#[test]
fn the_classifier_distinguishes_two_libraries() {
    // lwjgl and lwjgl:natives-linux are two files, both required.
    assert_ne!(
        library_key("org.lwjgl:lwjgl:3.3.3"),
        library_key("org.lwjgl:lwjgl:3.3.3:natives-linux")
    );
}

#[test]
fn a_name_without_a_colon_stays_itself() {
    assert_eq!(library_key("weird"), "weird");
}

/// A loader's descriptor only holds a delta and points to its base via
/// `inheritsFrom`: the chain must be walked up to get the whole thing.
#[test]
fn the_chain_walks_up_from_the_loader_to_the_base() {
    let tree = Tree::new("chain");
    tree.version("1.21.1", VANILLA)
        .version("neoforge-21.1.250", NEOFORGE);

    let chain = resolve_chain(&tree.shared(), "neoforge-21.1.250").unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(chain[0].id, "neoforge-21.1.250");
    assert_eq!(chain[1].id, "1.21.1");
    // The loader doesn't declare an assets index: the base carries it.
    assert!(chain[0].asset_index.is_none());
    assert!(chain[1].asset_index.is_some());
}

#[test]
fn a_lone_vanilla_version_forms_a_one_link_chain() {
    let tree = Tree::new("lone-chain");
    tree.version("1.21.1", VANILLA);

    let chain = resolve_chain(&tree.shared(), "1.21.1").unwrap();
    assert_eq!(chain.len(), 1);
}

/// The message must name the missing file: it's the one that needs
/// reinstalling, and a player won't guess which one.
#[test]
fn a_missing_version_names_the_expected_file() {
    let tree = Tree::new("missing-chain");
    let error = resolve_chain(&tree.shared(), "1.21.1").expect_err("nothing is installed");

    let text = format!("{error:#}");
    assert!(text.contains("1.21.1 is not installed"), "{text}");
    assert!(text.contains("1.21.1.json"), "{text}");
}

#[test]
fn an_unreadable_descriptor_is_reported_as_such() {
    let tree = Tree::new("broken-chain");
    tree.version("1.21.1", "{this isn't JSON");

    let error = resolve_chain(&tree.shared(), "1.21.1").expect_err("invalid JSON");
    assert!(format!("{error:#}").contains("unreadable"), "{error:#}");
}

/// An inheritance loop — or an absurdly deep chain — must not make the
/// launcher spin forever.
#[test]
fn a_chain_that_loops_stops() {
    let tree = Tree::new("looping-chain");
    tree.version("a", r#"{"id":"a","inheritsFrom":"b"}"#)
        .version("b", r#"{"id":"b","inheritsFrom":"a"}"#);

    let error = resolve_chain(&tree.shared(), "a").expect_err("the chain loops");
    assert!(format!("{error:#}").contains("too deep"), "{error:#}");
}
