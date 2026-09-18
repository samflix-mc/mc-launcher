use super::{HEIGHT, LABEL, ROUTE, SESSION_EVENT, WIDTH, sign_in_in_progress};

/// **The dimensions come from the design system, and they're checked.**
///
/// 440 × 520 is what its AuthWindow spec sets. A number copied wrong
/// doesn't show up in a review — it's a window a bit too narrow, and you
/// end up wondering afterward why the Microsoft button's label wraps.
#[test]
fn the_window_has_the_design_system_dimensions() {
    assert_eq!((WIDTH, HEIGHT), (440.0, 520.0));
}

/// The label is written on BOTH sides: here, and in `capabilities`, without
/// which this window's title bar buttons are refused by the ACL — and that
/// refusal only shows up in a packaged build.
#[test]
fn the_label_is_the_one_the_capability_declares() {
    let capability = include_str!("../capabilities/default.json");

    assert_eq!(LABEL, "signin");
    assert!(
        capability.contains("\"signin\""),
        "capabilities/default.json does not cover the \"{LABEL}\" window"
    );
}

/// The loaded route is a ROUTER route, not a file.
///
/// It relies on the asset protocol's fallback, which serves `index.html`
/// for an unknown path. Writing it `signin.html` would load a static file
/// — which was tried, and rendered halfway: the design system isn't copied
/// by hand, and the Microsoft button had neither the size nor the shape of
/// its siblings, with neither frosted glass nor an image behind it.
#[test]
fn the_route_is_the_router_s() {
    assert_eq!(ROUTE, "/signin");
    assert!(!ROUTE.ends_with(".html"));
    assert!(
        ROUTE.starts_with('/'),
        "the SPA fallback expects an absolute path"
    );
}

/// At rest, no sign-in is in progress: that's what allows
/// `startup::complete` to show the main window.
#[test]
fn at_rest_no_sign_in_is_in_progress() {
    assert!(!sign_in_in_progress());
}

/// The event's name is a contract with the front, written on both sides.
#[test]
fn the_session_event_carries_the_name_the_front_listens_for() {
    assert_eq!(SESSION_EVENT, "session-opened");
}
