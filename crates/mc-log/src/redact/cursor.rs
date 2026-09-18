//! Where a sensitive value starts and ends in a text.

/// Can a character belong to a token value?
///
/// The tokens matched here are base64url, JWT, or hex, plus the handful
/// of punctuation marks a JWT contains. Everything else — space, quote,
/// comma, brace — closes the value.
pub(super) fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '=' | '~' | '$')
}

/// Does a character separate a keyword from its value?
pub(super) fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, ':' | '=' | '"' | '\'' | ',')
}

/// End of the punctuation run starting at `from`.
pub(super) fn end_of_separators(text: &str, from: usize) -> usize {
    let rest = &text[from..];
    from + rest.find(|c: char| !is_separator(c)).unwrap_or(rest.len())
}

/// End of the value starting at `from`.
pub(super) fn end_of_value(text: &str, from: usize) -> usize {
    let rest = &text[from..];
    from + rest.find(|c: char| !is_token_char(c)).unwrap_or(rest.len())
}
