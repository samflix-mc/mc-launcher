use super::{absolute_image, acceptable, same_origin};

const FEED: &str = "https://mc-launcher.ggy.info/pack/news.json";

/// THE test of this module, and it's demonstrated rather than reasoned:
/// `https://mc-launcher.ggy.info.evil.example/` DOES start with
/// `https://mc-launcher.ggy.info`. A prefix comparison would accept it.
#[test]
fn a_host_that_starts_with_the_right_name_is_not_the_right_one() {
    for impostor in [
        "https://mc-launcher.ggy.info.evil.example/i.webp",
        "https://mc-launcher.ggy.infoo/i.webp",
        "https://evil.example/mc-launcher.ggy.info/i.webp",
    ] {
        assert!(!same_origin(impostor, FEED), "wrongly accepted: {impostor}");
    }
}

/// A `user@host`: what matters is AFTER the at-sign. Without this split,
/// `https://mc-launcher.ggy.info@evil.example/` would pass for our host,
/// because the start of the authority looks like the right name.
#[test]
fn an_authority_with_an_at_sign_does_not_fool_it() {
    assert!(!same_origin(
        "https://mc-launcher.ggy.info@evil.example/i.webp",
        FEED
    ));
}

#[test]
fn the_same_host_passes() {
    assert!(same_origin("https://mc-launcher.ggy.info/a/b.webp", FEED));
    // The host's case doesn't matter: DNS doesn't distinguish it.
    assert!(same_origin("https://MC-Launcher.GGY.info/b.webp", FEED));
}

/// The scheme matters too: serving an image in the clear from an HTTPS feed
/// would trigger a mixed-content warning, and above all reopen a door we
/// just closed.
#[test]
fn a_different_scheme_does_not_pass() {
    assert!(!same_origin("http://mc-launcher.ggy.info/i.webp", FEED));
}

/// And the scheme is NOT pinned to `https:`: the test server only serves
/// `http://127.0.0.1:<port>`, and pinning https would make the entire
/// network half of this crate untestable — a module that can't be exercised
/// ends up containing what we didn't want.
#[test]
fn a_local_plain_feed_stays_consistent_with_itself() {
    let local = "http://127.0.0.1:8080/pack/news.json";
    assert!(same_origin("http://127.0.0.1:8080/i.webp", local));
    assert!(!same_origin("http://127.0.0.1:9090/i.webp", local));
}

// --- Images ------------------------------------------------------------

#[test]
fn a_relative_image_resolves_against_the_feed() {
    assert_eq!(
        absolute_image("season3.webp", FEED).as_deref(),
        Some("https://mc-launcher.ggy.info/pack/season3.webp")
    );
    assert_eq!(
        absolute_image("/season3.webp", FEED).as_deref(),
        Some("https://mc-launcher.ggy.info/pack/season3.webp")
    );
}

/// A `..` would climb out of the published directory. We refuse rather than
/// normalize: nothing legitimate needs it, and normalizing would require
/// reproducing URL resolution rules, which have their own pitfalls.
#[test]
fn an_image_that_climbs_out_is_refused() {
    assert_eq!(absolute_image("../secret/i.webp", FEED), None);
    assert_eq!(absolute_image("a/../../i.webp", FEED), None);
}

#[test]
fn an_absolute_image_from_elsewhere_is_refused() {
    assert_eq!(absolute_image("https://evil.example/i.webp", FEED), None);
    assert_eq!(
        absolute_image("https://mc-launcher.ggy.info/i.webp", FEED).as_deref(),
        Some("https://mc-launcher.ggy.info/i.webp")
    );
}

// --- Links ---------------------------------------------------------------

/// A link is allowed to point elsewhere: that's what a link is for, and it's
/// never followed INSIDE the window — the front hands it to the system
/// browser.
#[test]
fn a_link_to_the_outside_is_accepted() {
    for href in [
        "https://modrinth.com/mod/jei",
        "http://example.invalid/",
        "mailto:contact@example.invalid",
    ] {
        assert!(acceptable(href, FEED).is_some(), "wrongly refused: {href}");
    }
}

/// Whatever makes no sense in a post, and has everything to gain from being
/// refused.
#[test]
fn dangerous_schemes_are_refused() {
    for href in [
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "file:///etc/passwd",
        "vbscript:msgbox",
        "/relative/path",
        "",
    ] {
        assert!(acceptable(href, FEED).is_none(), "wrongly accepted: {href}");
    }
}

/// The scheme's case doesn't save a dangerous link: `JavaScript:` would
/// otherwise slip through the cracks, and it's the oldest trick in the book.
#[test]
fn scheme_casing_does_not_save_a_dangerous_link() {
    for href in ["JavaScript:alert(1)", "JAVASCRIPT:alert(1)", "DaTa:x"] {
        assert!(acceptable(href, FEED).is_none(), "wrongly accepted: {href}");
    }
}

/// Surrounding spaces must not be enough to bypass the check.
#[test]
fn spaces_do_not_save_a_dangerous_link() {
    assert!(acceptable("  javascript:alert(1)  ", FEED).is_none());
    assert!(acceptable("  https://example.invalid/  ", FEED).is_some());
}
