use crate::redact::{MASK, redact};

#[test]
fn a_jwt_is_masked_even_without_a_keyword() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk";
    let output = redact(&format!("failure with {jwt} up front"));
    assert!(!output.contains("eyJhbGci"));
    assert!(output.contains(MASK));
    assert!(output.contains("failure with"));
}

#[test]
fn a_digest_is_not_mistaken_for_a_secret() {
    // SHA-1 digests are useful for diagnostics and reveal nothing.
    let text = "digest 88ee316e68900080b017f60c12162e2731924cf8 expected";
    assert_eq!(redact(text), text);
}

#[test]
fn a_short_identifier_starting_with_ey_survives() {
    // "eyZ2YBGT" is a Modrinth version identifier, not a token.
    let text = "pinned build eyZ2YBGT not found";
    assert_eq!(redact(text), text);
}

/// It's the word's **length** that decides, not where it falls in the
/// line. A short identifier starting with "eyJ" — the exact prefix of a
/// JWT — must survive no matter how far from the start it appears:
/// otherwise scrubbing would depend on what was written before it, and
/// the same message would be masked or not depending on the length of
/// its preamble.
#[test]
fn a_short_eyj_does_not_get_masked_just_because_it_appears_late() {
    let text = "configuration read from eyJcfg42";
    assert_eq!(redact(text), text);
}
