use super::{Check, Fetched};
use crate::Checksum;

fn file(name: &str, content: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mc-dl-check-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&path, content).unwrap();
    path
}

fn sha1_of(bytes: &[u8]) -> Checksum {
    Checksum::Sha1(Checksum::Sha1(String::new()).of(bytes))
}

#[test]
fn only_checks_that_have_one_expose_their_checksum() {
    let sum = sha1_of(b"x");
    assert!(Check::Full(&sum).checksum().is_some());
    assert!(Check::Quick { sum: &sum, size: 1 }.checksum().is_some());
    assert!(Check::Size(1).checksum().is_none());
    assert!(Check::Presence.checksum().is_none());
}

#[test]
fn only_size_checks_expose_a_size() {
    let sum = sha1_of(b"x");
    assert_eq!(Check::Quick { sum: &sum, size: 7 }.size(), Some(7));
    assert_eq!(Check::Size(9).size(), Some(9));
    assert_eq!(Check::Full(&sum).size(), None);
    assert_eq!(Check::Presence.size(), None);
}

/// For what gets executed — jars, libraries, runtimes — the digest is
/// recomputed on every pass.
#[test]
fn the_full_check_rereads_the_file() {
    let path = file("full", b"the real jar");

    assert!(Check::Full(&sha1_of(b"the real jar")).accepts_existing(&path));
    assert!(!Check::Full(&sha1_of(b"something else")).accepts_existing(&path));

    std::fs::remove_file(&path).ok();
}

/// A truncated asset has the wrong size; a silent corruption at a constant
/// size gives at worst a wrong texture, never executed code.
#[test]
fn the_quick_check_only_looks_at_the_size() {
    let path = file("quick", b"12345");
    let sum = sha1_of(b"unrelated");

    assert!(Check::Quick { sum: &sum, size: 5 }.accepts_existing(&path));
    assert!(!Check::Quick { sum: &sum, size: 6 }.accepts_existing(&path));
    assert!(Check::Size(5).accepts_existing(&path));

    std::fs::remove_file(&path).ok();
}

#[test]
fn presence_accepts_anything_that_exists() {
    let path = file("presence", b"whatever");
    assert!(Check::Presence.accepts_existing(&path));
    std::fs::remove_file(&path).ok();
}

/// A missing file is never accepted, whatever the strictness — except for
/// presence alone, which is only consulted after establishing it.
#[test]
fn an_unreadable_file_is_never_accepted() {
    let missing = std::env::temp_dir().join("mc-dl-file-that-does-not-exist");
    let sum = sha1_of(b"x");

    assert!(!Check::Full(&sum).accepts_existing(&missing));
    assert!(!Check::Quick { sum: &sum, size: 1 }.accepts_existing(&missing));
    assert!(!Check::Size(1).accepts_existing(&missing));
}

#[test]
fn the_two_outcomes_of_a_download_are_distinguishable() {
    // The install report contrasts them: "7 downloaded, 2,493 already
    // present" isn't the same information as "2,500 downloaded".
    assert_ne!(Fetched::Downloaded, Fetched::AlreadyPresent);
}
