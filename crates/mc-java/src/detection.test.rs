use super::detect;
use crate::fixtures::{MISSING_MAJOR, Tree, fake_java, silent_java};
use crate::version::Origin;

/// The managed runtime comes first: if it's there, the launcher installed
/// and verified it, no need to probe the system.
#[cfg(unix)]
#[tokio::test]
async fn the_managed_runtime_is_retained_and_recognized_as_such() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("detect-managed");
    fake_java(
        &tree.root.join("temurin-21").join("bin").join("java"),
        "21.0.5+11",
    );

    let java = detect(21, &tree.root).await.expect("it's there");

    assert_eq!(java.origin, Origin::Managed);
    assert_eq!(java.version.major, 21);
}

/// Below the required version, the game stops on
/// `UnsupportedClassVersionError` before even showing a window: a runtime
/// too old is no better than no runtime at all.
#[cfg(unix)]
#[tokio::test]
async fn a_runtime_too_old_is_not_retained() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("detect-old");
    fake_java(
        &tree
            .root
            .join(format!("temurin-{MISSING_MAJOR}"))
            .join("bin")
            .join("java"),
        "17.0.9",
    );

    assert!(detect(MISSING_MAJOR, &tree.root).await.is_none());
}

/// A runtime newer than requested does NOT qualify.
///
/// This is a reversal of the rule, and it's justified: the lock carries the
/// major version NeoForge was installed with. A newer Java changes mixin
/// behavior and the registry format, and the server cuts it off with an
/// ejection that doesn't name its cause. Accepting "at least" let the
/// player's machine choose what the pack had pinned.
///
/// This test carried the old rule, and carried it well: it's kept and
/// reversed rather than deleted, so the reversal is read rather than mistaken
/// for an oversight.
///
/// **The major requested is [`MISSING_MAJOR`], and that isn't a comfort
/// detail.** The first version of this test asked for 17 and faked a Java
/// announcing 21: it passed on this machine and FAILED on a runner, which
/// ships several JDKs — including a real 17. `detect` found it via `PATH`,
/// rightly retained it, and the test blamed the code for a flaw that was its
/// own. That's exactly the trap the comment on `MISSING_MAJOR` describes,
/// and which this test had escaped.
#[cfg(unix)]
#[tokio::test]
async fn a_runtime_newer_than_requested_does_not_qualify() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("detect-newer");
    fake_java(
        &tree
            .root
            .join(format!("temurin-{MISSING_MAJOR}"))
            .join("bin")
            .join("java"),
        &format!("{}.0.5+11", MISSING_MAJOR + 1),
    );

    assert!(
        detect(MISSING_MAJOR, &tree.root).await.is_none(),
        "a Java {} was retained where the pack requires exactly {MISSING_MAJOR}",
        MISSING_MAJOR + 1
    );
}

/// And the requested Java is indeed retained: hardening the rule must not
/// make detection inoperative.
#[cfg(unix)]
#[tokio::test]
async fn the_runtime_of_the_exact_major_is_retained() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("detect-exact");
    fake_java(
        &tree.root.join("temurin-17").join("bin").join("java"),
        "17.0.9+9",
    );

    let java = detect(17, &tree.root).await.expect("17 is indeed there");
    assert_eq!(java.version.major, 17);
}

/// An executable can be present and broken — dead symlink, half-uninstalled
/// package. Only what responds is retained.
#[cfg(unix)]
#[tokio::test]
async fn a_broken_binary_is_skipped_without_stopping_the_search() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("detect-broken");
    silent_java(
        &tree
            .root
            .join(format!("temurin-{MISSING_MAJOR}"))
            .join("bin")
            .join("java"),
    );

    // It responds nothing usable: detection continues, and finds nothing
    // else to offer instead.
    assert!(detect(MISSING_MAJOR, &tree.root).await.is_none());
}

#[tokio::test]
async fn an_empty_directory_yields_nothing() {
    let _workshop = crate::fixtures::workshop();
    let tree = Tree::new("detect-empty");
    assert!(detect(MISSING_MAJOR, &tree.root).await.is_none());
}
