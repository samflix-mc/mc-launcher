//! The three dead ends, and what they tell the player.

use super::{Request, decide};
use crate::resolve::fixtures::candidate;

/// Both sources answered and neither knows this project. Without a key, the
/// CurseForge keyword search is closed: a slug that doesn't match the one on
/// the site can't be found there, and that's the most frequent cause. The
/// message must send the reader to check the slug, not to hunt for a
/// failure.
#[test]
fn unfindable_project_sends_to_check_the_slug() {
    let error =
        decide(Vec::new(), &Request::new("jade"), "1.21.1", "neoforge").expect_err("no candidate");

    let text = error.to_string();
    assert!(
        text.contains("not found for Minecraft 1.21.1 / neoforge"),
        "{text}"
    );
    assert!(text.contains("slug"), "{text}");
}

#[test]
fn forbidden_download_points_to_the_mod_page() {
    let mut forbidden = candidate("jade", "15.10.6");
    forbidden.redistributable = false;
    let error = decide(vec![forbidden], &Request::new("jade"), "1.21.1", "neoforge")
        .expect_err("not redistributable");
    let text = error.to_string();
    assert!(text.contains("manual imports"), "{text}");
    assert!(text.contains("https://modrinth.com/mod/jade"), "{text}");
}

/// An empty URL is as good as an impossible download: the distinction only
/// shows at download time, too late to explain it.
#[test]
fn empty_url_counts_as_forbidden_download() {
    let mut no_url = candidate("jade", "15.10.6");
    no_url.url = String::new();
    let error =
        decide(vec![no_url], &Request::new("jade"), "1.21.1", "neoforge").expect_err("empty url");
    assert!(error.to_string().contains("manual imports"));
}
