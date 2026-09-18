use super::{report, requested};

/// The flag is recognized wherever it sits: a runner's launcher sometimes
/// passes the binary's path, sometimes graphical session arguments before
/// ours.
#[test]
fn the_flag_is_recognized_at_any_position() {
    assert!(requested(["--diagnostic"]));
    assert!(requested(["helm", "--diagnostic"]));
    assert!(requested(["--diagnostic", "--other"]));
}

/// And nothing else triggers it. A player who opens the launcher has no
/// arguments; a word that looks like it must not steal away their window.
#[test]
fn nothing_else_triggers_the_diagnostic() {
    assert!(!requested(Vec::<String>::new()));
    assert!(!requested(["helm"]));
    assert!(!requested(["--diagnostics"]));
    assert!(!requested(["diagnostic"]));
    assert!(!requested(["--diagnostic=1"]));
}

/// The report carries the four lines CI compares against what it built.
/// Look for them by their label and not by their rank: a line added at the
/// top must not break the check.
#[test]
fn the_report_names_what_ci_checks() {
    let text = report(false);

    for expected in ["name", "version", "environment", "directory"] {
        assert!(
            text.contains(expected),
            "\"{expected}\" missing from the report:\n{text}"
        );
    }
    // The version is the binary's, not a literal: it's what release.yml's
    // "coherence" job compares against the tag.
    assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
}

/// The rendering state reads both ways. It's the first line to ask whoever
/// sees a blank window under NVIDIA, and a fixed value wouldn't teach
/// anything.
#[test]
fn the_report_says_what_was_decided_about_rendering() {
    assert!(report(true).contains("disabled"), "{}", report(true));
    assert!(
        report(false).contains("left to WebKit"),
        "{}",
        report(false)
    );
}
