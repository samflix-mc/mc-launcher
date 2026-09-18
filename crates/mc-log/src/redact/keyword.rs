//! What a keyword announces.

use super::MASK;
use super::cursor::{end_of_separators, end_of_value};
use super::tables::{KEYWORDS, SCHEMES};

/// Masks what follows a sensitive keyword.
///
/// Covers `token=abc`, `"access_token": "abc"`, `Authorization: Bearer
/// abc` and `--api-key abc` with a single rule: after the keyword, skip
/// the separators and usual punctuation, then erase the value.
pub(super) fn redact_after_keywords(text: &str) -> String {
    // Keywords are pure ASCII and `to_ascii_lowercase` never changes
    // length: `lower`'s byte indices are exactly `text`'s. Comparing the
    // two `&str` directly avoids the two `Vec<char>` of the whole line,
    // plus the twenty that splitting the tables used to cost — and this
    // function sees every line written to the log.
    let lower = text.to_ascii_lowercase();
    // The common case, by far: one log line in a thousand carries a
    // keyword. The rest have nothing to copy.
    if !KEYWORDS.iter().any(|keyword| lower.contains(keyword)) {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    'outer: while i < text.len() {
        for keyword in KEYWORDS {
            if !lower[i..].starts_with(keyword) {
                continue;
            }
            // Separators between the keyword and its value.
            let mut j = end_of_separators(text, i + keyword.len());

            // The scheme announces the value: it stays readable and we
            // move past what it introduces. Under two conditions,
            // without which the mask would land in the wrong place: it
            // must form a word on its own, and it must actually
            // introduce something.
            for scheme in SCHEMES {
                if !lower[j..].starts_with(scheme) {
                    continue;
                }
                let after = end_of_separators(text, j + scheme.len());
                if after > j + scheme.len() && end_of_value(text, after) > after {
                    j = after;
                }
                break;
            }

            // The keyword and whatever comes with it stay readable:
            // without them, there'd be no way to tell which secret it
            // was.
            out.push_str(&text[i..j]);
            let end = end_of_value(text, j);
            if end > j {
                out.push_str(MASK);
            }
            i = end;
            continue 'outer;
        }
        let Some(c) = text[i..].chars().next() else {
            break;
        };
        out.push(c);
        i += c.len_utf8();
    }
    out
}

#[cfg(test)]
#[path = "keyword.test.rs"]
mod tests;
