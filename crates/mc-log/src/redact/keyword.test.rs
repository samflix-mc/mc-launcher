use crate::redact::{MASK, redact};

#[test]
fn the_usual_shapes_of_token_are_covered() {
    for entry in [
        "access_token=ya29.A0ARrdaM9xQ",
        "\"refresh_token\": \"M.C123_BAY.0.U.ArFbK\"",
        "Authorization: Bearer abcdef123456",
        "x-api-key: $2a$10$abcdefghijklmnop",
        // Made-up UUID: a real secret has no business in a test, it
        // would end up versioned forever.
        "--api-key 00000000-1111-2222-3333-444444444444",
    ] {
        let output = redact(entry);
        assert!(output.contains(MASK), "not masked: {entry} → {output}");
    }
}

#[test]
fn an_auth_scheme_other_than_bearer_does_not_let_the_value_through() {
    // "Basic" carries a username/password pair in base64: it's the most
    // sensitive content this header can carry. The scheme name itself
    // must survive: it's all that's left to tell which header it was.
    for (entry, expected) in [
        (
            "Authorization: Basic dXNlcjpwYXNzd29yZA==",
            "Authorization: Basic [secret]",
        ),
        (
            "Authorization: Digest cnonce=abcdef123456",
            "Authorization: Digest [secret]",
        ),
        (
            "authorization: Token abcdef123456",
            "authorization: Token [secret]",
        ),
        // Quoted value: without punctuation allowed after the scheme,
        // the line came out unchanged — without even a mask to flag it.
        (
            "Authorization: Basic \"dXNlcjpwYXNzd29yZA==\"",
            "Authorization: Basic \"[secret]\"",
        ),
    ] {
        assert_eq!(redact(entry), expected, "entry: {entry}");
    }
}

#[test]
fn a_secret_starting_like_a_scheme_stays_masked_in_full() {
    // A scheme is only recognized as one if it forms a word on its own
    // and actually introduces something. Otherwise it is the value, and
    // recognizing it as a scheme would move the mask past the first
    // characters of the secret.
    for (entry, expected) in [
        ("token=basicSECRETVALUE", "token=[secret]"),
        ("password=dpop9f3a2b", "password=[secret]"),
        ("secret=token12345", "secret=[secret]"),
        ("api_key=bearerAAAA1111", "api_key=[secret]"),
        ("password: digest", "password: [secret]"),
        ("Authorization: Bearer", "Authorization: [secret]"),
    ] {
        assert_eq!(redact(entry), expected, "entry: {entry}");
    }
}

/// A keyword with no value behind it hides nothing: placing a mask would
/// announce a secret where there is none, and make a log misleading —
/// leading someone to search for a leak that never happened.
#[test]
fn a_keyword_that_announces_nothing_is_not_masked() {
    for text in [
        "token=",
        "api_key: ",
        "Authorization:",
        "lost my credentials, ask around for the secret",
    ] {
        assert_eq!(redact(text), text, "entry: {text}");
    }
}

/// The scheme only gives way to what it introduces if it introduces
/// something. A "Bearer" followed by blank space at the end of the line
/// introduces nothing: it's the scheme itself that must be masked,
/// otherwise the line would come out unchanged.
#[test]
fn a_scheme_followed_by_nothing_stays_the_value_to_mask() {
    assert_eq!(redact("authorization: bearer "), "authorization: [secret] ");
    assert_eq!(
        redact("Authorization: Bearer\n"),
        "Authorization: [secret]\n"
    );
}
