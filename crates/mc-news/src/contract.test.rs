use super::{Post, is_valid_date, sort};

fn post(id: &str, date: &str, pinned: bool) -> Post {
    Post {
        id: id.into(),
        title: id.into(),
        date: date.into(),
        pinned,
        image: None,
        body: Vec::new(),
    }
}

fn ids(posts: &[Post]) -> Vec<&str> {
    posts.iter().map(|b| b.id.as_str()).collect()
}

/// Pinned first, then most recent to oldest.
///
/// The sort is in Rust and not in an Angular `computed()`, for the same
/// reason as the button rule: a rule that lives on the front is only checked
/// by vitest and falls outside mutation coverage. This one is exactly the
/// kind whose inversion breaks no display test and shows up only to the eye,
/// weeks later.
#[test]
fn pinned_first_then_most_recent_to_oldest() {
    let sorted = sort(vec![
        post("old", "2026-01-01T00:00:00Z", false),
        post("pinned-old", "2025-01-01T00:00:00Z", true),
        post("recent", "2026-09-01T00:00:00Z", false),
        post("pinned-recent", "2026-08-01T00:00:00Z", true),
    ]);

    assert_eq!(
        ids(&sorted),
        vec!["pinned-recent", "pinned-old", "recent", "old"]
    );
}

/// At equal date and pinning, the id decides.
///
/// Without this last criterion, the order would depend on that of the
/// received JSON, and two loads of the same feed might not give the same
/// screen — which is barely noticeable, and diagnoses very poorly.
#[test]
fn ties_are_broken_by_the_id() {
    let one = sort(vec![
        post("b", "2026-09-01T00:00:00Z", false),
        post("a", "2026-09-01T00:00:00Z", false),
    ]);
    let two = sort(vec![
        post("a", "2026-09-01T00:00:00Z", false),
        post("b", "2026-09-01T00:00:00Z", false),
    ]);
    assert_eq!(ids(&one), vec!["a", "b"]);
    assert_eq!(ids(&one), ids(&two));
}

#[test]
fn an_empty_feed_sorts_without_panicking() {
    assert!(sort(Vec::new()).is_empty());
}

// --- The date ----------------------------------------------------------

#[test]
fn an_rfc3339_utc_date_is_valid() {
    assert!(is_valid_date("2026-09-18T18:00:00Z"));
    assert!(is_valid_date("1999-12-31T23:59:59Z"));
}

/// A timezone offset is REFUSED, and that isn't rigidity: the sort compares
/// the strings as-is, which is only chronological order if all of them are
/// in UTC with the same number of digits. A date in "+02:00" would break
/// that property, and nothing in the display would show it — the posts
/// would simply be in a slightly wrong order.
#[test]
fn a_timezone_offset_is_refused() {
    assert!(!is_valid_date("2026-09-18T18:00:00+02:00"));
    assert!(!is_valid_date("2026-09-18T18:00:00-05:00"));
}

#[test]
fn malformed_dates_are_refused() {
    for bad in [
        "",
        "2026-09-18",
        "2026-09-18 18:00:00Z",   // space instead of T
        "2026/09/18T18:00:00Z",   // separators
        "26-09-18T18:00:00Z",     // two-digit year
        "2026-09-18T18:00:00",    // no Z
        "2026-09-18T18:00:00.5Z", // fraction of a second
        "aaaa-bb-ccTdd:ee:ffZ",
    ] {
        assert!(!is_valid_date(bad), "wrongly accepted: \"{bad}\"");
    }
}
