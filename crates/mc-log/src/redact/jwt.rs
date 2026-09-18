//! Tokens recognizable by shape alone.

use super::MASK;
use super::cursor::is_token_char;

/// Masks JWT-shaped tokens.
///
/// Microsoft and Minecraft tokens are among them: three base64url
/// segments separated by dots, starting with `eyJ` — the encoding of
/// `{"`. That prefix is enough to recognize them without regard to
/// context, which also catches tokens that no keyword introduces.
pub(super) fn redact_jwt(text: &str) -> String {
    // No token without this prefix: ruling it out first avoids copying
    // every line of the log into a `Vec<char>` to find nothing.
    if !text.contains("eyJ") {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i..].starts_with(&['e', 'y', 'J']) {
            let mut end = i;
            while end < chars.len() && is_token_char(chars[end]) {
                end += 1;
            }
            // An identifier starting with "eyJ" without being a token is
            // too short to be one: a JWT is always considerably longer.
            if end - i >= 24 {
                out.push_str(MASK);
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
#[path = "jwt.test.rs"]
mod tests;
