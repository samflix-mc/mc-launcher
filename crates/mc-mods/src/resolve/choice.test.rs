use super::{Request, decide};
use crate::Channel;
use crate::resolve::fixtures::candidate;

#[test]
fn the_kept_build_is_returned_as_is() {
    let kept = decide(
        vec![candidate("jade", "15.10.6")],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
    )
    .expect("a kept build");
    assert_eq!(kept.version_number, "15.10.6");
}

#[test]
fn missing_requested_version_says_so_rather_than_not_found() {
    let mut request = Request::new("jade");
    request.version = Some("99.0.0".into());
    let error = decide(
        vec![candidate("jade", "15.10.6")],
        &request,
        "1.21.1",
        "neoforge",
    )
    .expect_err("missing version");
    let text = error.to_string();
    assert!(text.contains("no version matches"), "{text}");
    assert!(text.contains("\"99.0.0\""), "{text}");
}

#[test]
fn a_too_strict_channel_names_the_channel() {
    let mut candidates = vec![candidate("jade", "15.10.6")];
    candidates[0].channel = Channel::Beta;
    let mut request = Request::new("jade");
    request.channel = Some(Channel::Release);
    let error = decide(candidates, &request, "1.21.1", "neoforge").expect_err("no release");
    assert!(error.to_string().contains("the release channel"), "{error}");
}
