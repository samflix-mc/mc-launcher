use crate::redact::{MASK, redact};

#[test]
fn the_field_name_stays_readable() {
    // Without the keyword, an incident would no longer say which token it was.
    let output = redact("refresh_token=M.C123_BAY");
    assert!(output.starts_with("refresh_token="));
    assert!(output.ends_with(MASK));
}

#[test]
fn a_text_without_a_secret_is_untouched() {
    let text = "download of jei-1.21.1-neoforge-19.51.0.418.jar (1.7 MiB)";
    assert_eq!(redact(text), text);
}

#[test]
fn several_secrets_in_the_same_text() {
    let output = redact("token=abc123456 then api_key=def789012 end");
    assert_eq!(output.matches(MASK).count(), 2);
    assert!(output.contains("then"));
    assert!(output.ends_with("end"));
}
