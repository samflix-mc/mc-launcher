//! Which URLs a post is allowed to carry.
//!
//! ## Two rules, and two different dangers
//!
//! **Images** must come from the same host as the feed. They're downloaded by
//! Rust and displayed in the window: an image from elsewhere would bring in a
//! third party on every page open.
//!
//! **Links** are allowed to point elsewhere: that's what a link is for. They
//! are never followed inside the window — the front hands them to
//! `ouvrirPage()`, which passes them to the system browser, and the
//! navigation plugin would refuse anyway. What we filter on them is
//! therefore the SCHEME: `javascript:` and `data:` make no sense in a post,
//! and have everything to gain from being refused.

/// The one place that decides what "the same host" means.
///
/// ## The prefix trap
///
/// Comparing with `starts_with` on the whole URL is wrong, and demonstrably
/// so: `https://mc-launcher.ggy.info.evil.example/` does start with
/// `https://mc-launcher.ggy.info`. So we compare the EXTRACTED host, in
/// full, and the port with it.
///
/// ## Why the scheme isn't pinned to `https:`
///
/// `mc_testkit::Server` only serves `http://127.0.0.1:<port>`. Pinning
/// `https:` would make the entire network half of this crate untestable —
/// and a module that can't be exercised ends up containing what we didn't
/// want. The rule is therefore "same scheme AND same host as the feed",
/// which is stricter in production, where the feed is on `https:`.
pub fn same_origin(url: &str, feed_host: &str) -> bool {
    match (origin(url), origin(feed_host)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// `scheme://host[:port]`, without the path.
fn origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() {
        return None;
    }
    // A `user@host`: what matters is after the at-sign. Without this split,
    // `https://mc-launcher.ggy.info@evil.example/` would pass for our host.
    let host = authority.rsplit('@').next()?;
    if host.is_empty() {
        return None;
    }
    Some(format!(
        "{}://{}",
        scheme.to_ascii_lowercase(),
        host.to_ascii_lowercase()
    ))
}

/// A post image's absolute URL, or `None` if it falls outside the host.
///
/// `image` may be relative to the feed — that's the expected form — or
/// absolute, in which case it must be on the same host.
pub fn absolute_image(image: &str, feed_url: &str) -> Option<String> {
    if image.contains("://") {
        return same_origin(image, feed_url).then(|| image.to_string());
    }
    // Relative: resolve it against the feed's directory.
    let base = feed_url.rsplit_once('/')?.0;
    let clean = image.trim_start_matches('/');
    // A `..` would climb out of the published directory. We refuse rather
    // than normalize: nothing legitimate needs it.
    if clean.split('/').any(|part| part == "..") {
        return None;
    }
    Some(format!("{base}/{clean}"))
}

/// The schemes a post link is allowed to carry.
const LINK_SCHEMES: [&str; 3] = ["https://", "http://", "mailto:"];

/// An acceptable, normalized link URL.
///
/// Returns `None` for anything that isn't one of the schemes above:
/// `javascript:`, `data:`, `file:` make no sense in a post.
pub fn acceptable(href: &str, _feed_host: &str) -> Option<String> {
    let trimmed = href.trim();
    // The comparison is case-insensitive: `JavaScript:` would otherwise slip
    // through the cracks.
    let lowercase = trimmed.to_ascii_lowercase();
    LINK_SCHEMES
        .iter()
        .any(|scheme| lowercase.starts_with(scheme))
        .then(|| trimmed.to_string())
}

#[cfg(test)]
#[path = "links.test.rs"]
mod tests;
