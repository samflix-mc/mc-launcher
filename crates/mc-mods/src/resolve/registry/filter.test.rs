use super::{Channel, Request, check_compatible, pick};
use crate::resolve::fixtures::installed;

/// A pinned build escapes API filtering: no one else checks that it targets
/// the pack's loader. Placing a Fabric jar in a NeoForge pack gives a game
/// that won't start, and an error that doesn't say why.
#[test]
fn a_pinned_build_for_another_loader_is_refused() {
    let mut candidate = installed("jei", &["jei"], &[]).candidate;
    candidate.file_name = "jei-fabric-1.21.1-19.21.jar".into();

    let refusal = check_compatible(&candidate, "1.21.1", "neoforge", "jei")
        .expect_err("a Fabric jar has no business in a NeoForge pack");
    assert!(
        refusal.to_string().contains("different loader"),
        "unexpected message: {refusal}"
    );
}

/// Many mods publish one jar per loader under a name that cites them all.
/// Refusing it on the mere presence of the word "fabric" would rule out
/// perfectly valid builds — it's the absence of ours that condemns it.
#[test]
fn a_name_that_cites_both_loaders_passes() {
    let mut candidate = installed("jei", &["jei"], &[]).candidate;
    candidate.file_name = "jei-fabric-neoforge-1.21.1.jar".into();
    assert!(check_compatible(&candidate, "1.21.1", "neoforge", "jei").is_ok());

    // And a name that cites no loader isn't judged: the file name is too
    // unreliable to reject on a silence.
    let mut silent = installed("jei", &["jei"], &[]).candidate;
    silent.file_name = "jei-19.21.jar".into();
    assert!(check_compatible(&silent, "1.21.1", "neoforge", "jei").is_ok());
}

/// Pinning happens under whatever name is in front of us, and that name
/// isn't the same depending on the API: Modrinth shows a version number,
/// CurseForge a file name, the project page a title. All three must find the
/// same build.
#[test]
fn a_pinned_version_is_recognized_under_its_three_names() {
    let mut candidate = installed("jei", &["jei"], &[]).candidate;
    candidate.version_number = "19.21.0.247".into();
    candidate.file_name = "jei-1.21.1-neoforge-19.21.0.247.jar".into();
    candidate.display_name = "Just Enough Items 19.21".into();

    for name in [
        "19.21.0.247",
        "jei-1.21.1-neoforge-19.21.0.247.jar",
        "Just Enough Items 19.21",
        // Case must not decide: it varies from one page to another.
        "JEI-1.21.1-NEOFORGE-19.21.0.247.JAR",
    ] {
        let mut request = Request::new("jei");
        request.version = Some(name.to_string());
        assert!(
            pick(vec![candidate.clone()], &request).is_some(),
            "pin by \"{name}\" not recognized"
        );
    }
}

#[test]
fn the_most_recent_channel_is_retained() {
    let mut old = installed("jei", &["jei"], &[]).candidate;
    old.published = "2024-01-01".into();
    let mut recent = old.clone();
    recent.published = "2025-06-01".into();
    recent.version_number = "19.56".into();

    let choice = pick(vec![old, recent], &Request::new("jei")).unwrap();
    assert_eq!(choice.version_number, "19.56");
}

#[test]
fn a_beta_is_discarded_by_default() {
    let mut beta = installed("jei", &["jei"], &[]).candidate;
    beta.channel = Channel::Beta;
    assert!(pick(vec![beta.clone()], &Request::new("jei")).is_none());

    let mut request = Request::new("jei");
    request.channel = Some(Channel::Beta);
    assert!(pick(vec![beta], &request).is_some());
}

#[test]
fn a_pinned_version_that_does_not_exist_does_not_fall_back_to_anything() {
    // Silently falling back to another version would defeat the whole point
    // of pinning.
    let candidate = installed("jei", &["jei"], &[]).candidate;
    let mut request = Request::new("jei");
    request.version = Some("99.99".into());
    assert!(pick(vec![candidate], &request).is_none());
}
