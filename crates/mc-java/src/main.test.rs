use super::{Settings, parse};
use std::path::PathBuf;

fn parse_args(args: &[&str]) -> anyhow::Result<Settings> {
    parse(args.iter().map(|a| (*a).to_string()))
}

#[test]
fn with_no_argument_a_java_21_is_sought() {
    // Minecraft 1.21.1 requires it; below that the game stops before showing
    // a window.
    assert_eq!(parse_args(&[]).unwrap(), Settings::default());
    assert_eq!(parse_args(&[]).unwrap().major, 21);
}

#[test]
fn the_three_options_combine() {
    let settings = parse_args(&["--check", "--major", "17", "--dir", "/tmp/runtimes"]).unwrap();

    assert!(settings.check_only);
    assert_eq!(settings.major, 17);
    assert_eq!(settings.dir, Some(PathBuf::from("/tmp/runtimes")));
}

/// An unreadable value is a typo, not a bug: it must be announced as an
/// ordinary error, along with what was read.
#[test]
fn an_unreadable_major_is_reported_without_panicking() {
    let error = parse_args(&["--major", "twenty-one"]).expect_err("not an integer");
    assert!(format!("{error:#}").contains("twenty-one"), "{error:#}");
}

#[test]
fn an_option_that_expects_a_value_demands_it() {
    assert!(parse_args(&["--major"]).is_err());
    assert!(parse_args(&["--dir"]).is_err());
}

#[test]
fn an_unknown_option_is_named() {
    let error = parse_args(&["--bogus"]).expect_err("unknown option");
    assert!(format!("{error:#}").contains("--bogus"), "{error:#}");
}

/// `--check` touches nothing: it says whether this machine already has a
/// usable Java, and returns failure when there isn't one — that's what a CI
/// calls.
#[tokio::test]
async fn check_without_a_runtime_returns_failure_without_installing_anything() {
    let root = std::env::temp_dir().join(format!("mc-java-main-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();

    // A major version no system will provide, so this machine's PATH doesn't
    // muddy the result.
    let code = super::execute(999, true, Some(root.clone())).await.unwrap();

    assert_eq!(
        format!("{code:?}"),
        format!("{:?}", std::process::ExitCode::FAILURE)
    );
    assert!(
        !root.join("temurin-999").exists(),
        "nothing should be installed"
    );
    std::fs::remove_dir_all(&root).ok();
}
