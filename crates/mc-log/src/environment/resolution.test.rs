use super::resolve;
use crate::environment::Environment;

#[test]
fn without_a_declaration_we_stay_local() {
    // The case of a `cargo run --release` on a machine: it's not because
    // the profile is "release" that the deployment is production.
    assert_eq!(resolve(None, None), Environment::Local);
}

#[test]
fn compilation_sets_the_environment() {
    assert_eq!(resolve(None, Some("production")), Environment::Production);
    assert_eq!(resolve(None, Some("preprod")), Environment::Preproduction);
}

#[test]
fn launch_takes_priority_over_compilation() {
    // Replaying a production binary locally must not taint production.
    assert_eq!(
        resolve(Some("local"), Some("production")),
        Environment::Local
    );
}

#[test]
fn an_unknown_value_does_not_take_the_place_of_the_rest() {
    // A typo at launch must not erase what compilation had declared.
    assert_eq!(
        resolve(Some("prodction"), Some("production")),
        Environment::Production
    );
    assert_eq!(resolve(Some("whatever"), None), Environment::Local);
}

#[test]
fn the_usual_aliases_are_accepted() {
    for (text, expected) in [
        ("dev", Environment::Development),
        ("DEV", Environment::Development),
        ("staging", Environment::Preproduction),
        ("pre-prod", Environment::Preproduction),
        (" prod ", Environment::Production),
    ] {
        assert_eq!(Environment::parse(text), Some(expected), "for \"{text}\"");
    }
}

#[test]
fn only_published_environments_are_said_to_be_deployed() {
    assert!(!Environment::Local.is_deployed());
    assert!(!Environment::Development.is_deployed());
    assert!(Environment::Preproduction.is_deployed());
    assert!(Environment::Production.is_deployed());
}

/// `current` and `origin` read the same variable, and it's their agreement
/// that matters: a diagnostic announcing "production" and "default, no
/// declaration" on the same run sends you looking in the wrong place.
#[test]
fn the_diagnostic_says_where_the_environment_comes_from() {
    use super::{COMPILED, current, origin};

    let vars = crate::fixtures::variables();
    vars.set("SAMFLIX_ENV", "staging");
    assert_eq!(current(), Environment::Preproduction);
    assert_eq!(origin(), "SAMFLIX_ENV variable at launch");

    // An unreadable value must not be announced as a declaration: this is
    // precisely the case where one is looking for why the environment
    // isn't the one expected.
    vars.set("SAMFLIX_ENV", "prodction");
    assert_ne!(origin(), "SAMFLIX_ENV variable at launch");

    // Without a declaration at launch, all that's left is what compilation
    // managed to freeze: nothing on a machine, "development" on the CI,
    // which compiles with the variable set. This test states the agreement
    // between the two answers; it can't say which one, or it would be
    // testing the runner.
    vars.unset("SAMFLIX_ENV");
    match COMPILED.and_then(Environment::parse) {
        Some(compiled) => {
            assert_eq!(current(), compiled);
            assert_eq!(origin(), "SAMFLIX_ENV frozen at compile time");
        }
        None => {
            assert_eq!(current(), Environment::Local);
            assert_eq!(origin(), "default, no declaration");
        }
    }
}

#[test]
fn each_environment_has_the_name_sentry_expects() {
    for (env, name) in [
        (Environment::Local, "local"),
        (Environment::Development, "development"),
        (Environment::Preproduction, "preproduction"),
        (Environment::Production, "production"),
    ] {
        assert_eq!(env.as_str(), name);
        assert_eq!(Environment::parse(name), Some(env));
    }
}
