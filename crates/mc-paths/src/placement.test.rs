use super::{current, placed};

/// Without a placement, `current()` still answers — and answers with what
/// the environment says.
///
/// The crates are usable outside the application: `mc-pack` on the command
/// line never calls `place`, and yet must find its data.
///
/// This test lives in the same process as the crate's other tests, where NO
/// placement happens — the placement itself is exercised in
/// `tests/unique_placement.rs`, a separate binary. Two tests in the same
/// binary that each placed their own would contradict each other depending
/// on their execution order.
#[test]
fn without_a_placement_the_locations_come_from_the_environment() {
    assert!(!placed(), "no placement should happen in this binary");
    assert_eq!(current(), crate::from_system());
}

/// And the fallback doesn't memoize: two successive calls re-read the
/// environment.
///
/// Without this property, the first caller would freeze the tree for the
/// whole process — and seven existing suites, which move `XDG_DATA_HOME`
/// along the way, would see their move take effect or not depending on their
/// rank in the suite. A test that passes or fails depending on its rank is
/// worse than no test at all.
#[test]
fn the_fallback_does_not_memoize() {
    let before = current();

    // SAFETY: the variable is restored before the end of the test, and no
    // other test in this crate reads it.
    let previous = std::env::var_os("XDG_DATA_HOME");
    unsafe { std::env::set_var("XDG_DATA_HOME", "/elsewhere/for-the-fallback") };
    let during = current();
    unsafe {
        match previous {
            Some(value) => std::env::set_var("XDG_DATA_HOME", value),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }
    let after = current();

    // Under macOS and Windows, XDG has no effect: the test then only covers
    // the fact that nothing got frozen, which the other two equalities
    // attest to.
    if cfg!(all(unix, not(target_os = "macos"))) {
        assert_ne!(
            before, during,
            "the fallback froze the first caller's environment"
        );
        assert!(during.data.starts_with("/elsewhere/for-the-fallback"));
    }
    assert_eq!(before, after, "the environment was not restored");
}

/// The refusal to re-place READS clearly.
///
/// `AlreadyPlaced` is an error someone will see — in a log, on an error
/// output — at the moment they're looking for why the paths aren't what they
/// expected. Its `Display` could render an empty string without any test
/// noticing: the message would have disappeared while keeping the error,
/// which is the worse half of the two.
#[test]
fn the_refusal_to_re_place_says_why() {
    let message = super::AlreadyPlaced.to_string();

    assert!(!message.is_empty());
    assert!(message.contains("already"), "{message}");
    // And it says what to conclude, not just what happened.
    assert!(message.contains("replaced"), "{message}");
}
