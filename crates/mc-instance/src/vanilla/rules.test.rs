use super::{Features, allowed, allowed_with};
use crate::vanilla::descriptor::{OsCondition, Rule};

fn rule(action: &str, os: Option<&str>, arch: Option<&str>) -> Rule {
    Rule {
        action: action.to_string(),
        os: os.map(|name| OsCondition {
            name: Some(name.to_string()),
            arch: arch.map(str::to_string),
        }),
        features: None,
    }
}

/// Rule conditioned on a flag, like the ones on arguments.
fn feature_rule(action: &str, feature: &str, expected: bool) -> Rule {
    Rule {
        action: action.to_string(),
        os: None,
        features: Some(std::collections::BTreeMap::from([(
            feature.to_string(),
            expected,
        )])),
    }
}

#[test]
fn without_a_rule_the_library_is_kept() {
    assert!(allowed(&[], "linux", "x86_64"));
}

#[test]
fn an_argument_under_a_flag_only_appears_if_the_flag_is_active() {
    // This is what keeps --quickPlayMultiplayer absent unless joining a
    // server is actually requested.
    let rules = vec![feature_rule("allow", "is_quick_play_multiplayer", true)];
    let active = Features::from(["is_quick_play_multiplayer".to_string()]);

    assert!(allowed_with(&rules, "linux", "x86_64", &active));
    assert!(!allowed_with(&rules, "linux", "x86_64", &Features::new()));
}

#[test]
fn a_flag_expected_false_requires_its_absence() {
    // "except in demo": the rule applies when the flag is absent.
    let rules = vec![feature_rule("allow", "is_demo_user", false)];
    assert!(allowed_with(&rules, "linux", "x86_64", &Features::new()));

    let demo = Features::from(["is_demo_user".to_string()]);
    assert!(!allowed_with(&rules, "linux", "x86_64", &demo));
}

#[test]
fn library_rules_ignore_flags() {
    // They don't carry any: the behavior must stay the same as before.
    let rules = vec![rule("allow", Some("linux"), None)];
    assert!(allowed_with(&rules, "linux", "x86_64", &Features::new()));
    assert!(!allowed_with(&rules, "osx", "x86_64", &Features::new()));
}

#[test]
fn a_library_reserved_for_macos_is_excluded_elsewhere() {
    // java-objc-bridge only makes sense on macOS; installing it on Linux
    // weighs down the classpath without ever being used.
    let rules = vec![rule("allow", Some("osx"), None)];
    assert!(allowed(&rules, "osx", "x86_64"));
    assert!(!allowed(&rules, "linux", "x86_64"));
}

#[test]
fn the_last_applicable_rule_wins() {
    let rules = vec![
        rule("allow", None, None),
        rule("disallow", Some("osx"), None),
    ];
    assert!(allowed(&rules, "linux", "x86_64"));
    assert!(!allowed(&rules, "osx", "arm64"));
}

#[test]
fn a_rule_can_target_an_architecture() {
    let rules = vec![rule("allow", Some("windows"), Some("x86"))];
    assert!(allowed(&rules, "windows", "x86"));
    assert!(!allowed(&rules, "windows", "x86_64"));
}
