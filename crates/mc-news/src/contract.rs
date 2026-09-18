//! What the host publishes, and what the launcher makes of it.
//!
//! ## The contract, written here and nowhere else
//!
//! ```jsonc
//! {
//!   "schema": 1,
//!   "posts": [
//!     {
//!       "id": "2026-09-season-3",       // stable, serves as the front's key
//!       "title": "Season 3 opens",
//!       "date": "2026-09-18T18:00:00Z", // RFC 3339, in UTC
//!       "pinned": true,                 // optional, defaults to false
//!       "image": "season3.webp",        // optional, relative to the feed
//!       "body": "Text **markdown**."
//!     }
//!   ]
//! }
//! ```
//!
//! A faulty post is discarded ALONE: an unreadable date on one post must not
//! make the other nine disappear. That's the difference between "the news
//! page has a gap" and "the news page is empty", and the second reads like a
//! launcher failure.

use serde::{Deserialize, Serialize};

/// Version of the format served by the host.
pub const SCHEMA: u32 = 1;

/// What the host serves, as-is.
#[derive(Debug, Clone, Deserialize)]
pub struct RawFeed {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub posts: Vec<RawPost>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPost {
    pub id: String,
    pub title: String,
    /// RFC 3339. Parsed here rather than by the front: an invalid date must
    /// discard the post, and the front has no way to decide that.
    pub date: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub image: Option<String>,
    pub body: String,
}

/// A retained post, its body already parsed.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: String,
    pub title: String,
    pub date: String,
    pub pinned: bool,
    /// The image's absolute URL, already checked. `None` if there isn't one,
    /// or if the one that was announced fell outside the feed's host.
    pub image: Option<String>,
    /// The body, as a typed tree. **Never HTML**: that's the condition under
    /// which the CSP was loosened.
    pub body: Vec<crate::tree::Block>,
}

/// The feed, ready for the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    pub posts: Vec<Post>,
    /// The feed comes from the cache and not the network.
    pub offline: bool,
    /// How many posts were discarded, and why — for the log, and so that a
    /// half-broken feed gets noticed.
    pub discarded: Vec<String>,
}

/// Sorts and validates, in Rust.
///
/// ## Why the sort is here
///
/// The same reason as for the button rule: a rule that lives in an Angular
/// `computed()` is only checked by vitest, and falls outside mutation
/// coverage. This one — "pinned first, then most recent to oldest" — is
/// exactly the kind of rule whose inversion breaks no display test and shows
/// up only to the eye, weeks later.
///
/// The sort works on the RFC 3339 date as-is: its format is lexicographically
/// ordered, provided it's in UTC with the same number of digits — which
/// `validate_the_date` enforces.
pub fn sort(mut posts: Vec<Post>) -> Vec<Post> {
    posts.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.date.cmp(&a.date))
            // At equal date and pinning, the id decides: without it, the
            // order would depend on that of the received JSON, and two loads
            // of the same feed might not give the same screen.
            .then_with(|| a.id.cmp(&b.id))
    });
    posts
}

/// Is a date RFC 3339 in UTC, as required?
///
/// Deliberately STRICT check on the shape, not a full parse: we don't want a
/// date library just to validate what the host publishes, and the shape is
/// enough to guarantee what the sort needs — a string whose lexicographic
/// order is chronological order.
///
/// A timezone offset ("+02:00") is therefore REFUSED: it would break that
/// property, and nothing in the display would show it.
pub fn is_valid_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let digits = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    if !digits.iter().all(|&i| bytes[i].is_ascii_digit()) {
        return false;
    }
    bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
}

#[cfg(test)]
#[path = "contract.test.rs"]
mod tests;
