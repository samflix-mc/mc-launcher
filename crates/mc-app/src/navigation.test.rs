use super::allowed;
use tauri::Url;

fn url(raw: &str) -> Url {
    Url::parse(raw).expect("valid test URL")
}

/// The application's three origins, and there really are three: the asset
/// protocol's origin isn't the same everywhere. Forgetting one gives a
/// blank window on the forgotten platform only — that is, in practice, on
/// someone else's machine.
#[test]
fn the_application_s_origins_pass() {
    for raw in [
        // Linux and macOS.
        "tauri://localhost",
        "tauri://localhost/",
        "tauri://localhost/index.html",
        // Windows.
        "http://tauri.localhost/",
        "http://tauri.localhost/index.html",
        // `tauri dev`, where it's Angular's server that serves the page.
        "http://localhost:1420/",
        "http://localhost:1420/index.html",
    ] {
        assert!(allowed(&url(raw)), "wrongly refused: {raw}");
    }
}

/// The router uses `withHashLocation()`: all internal navigation happens
/// through the fragment. Refusing these URLs would freeze the application
/// on its first screen.
#[test]
fn fragment_based_navigation_passes() {
    for raw in [
        "tauri://localhost/#/spawn",
        "tauri://localhost/index.html#/news",
        "http://tauri.localhost/#/settings",
    ] {
        assert!(allowed(&url(raw)), "wrongly refused: {raw}");
    }
}

/// What this control exists to prevent: a link in a news post that would
/// make the whole window navigate to a remote site. This window is a
/// privileged origin where `invoke` is reachable; a site loaded there
/// would inherit the keyring and the file system.
#[test]
fn a_remote_site_is_refused() {
    for raw in [
        "https://exemple.invalid/",
        "http://mc-launcher.ggy.info/",
        "https://modrinth.com/mod/jei",
        "https://mc-heads.net/head/abc",
    ] {
        assert!(!allowed(&url(raw)), "wrongly accepted: {raw}");
    }
}

/// A host that STARTS WITH the right name isn't the right host. This is the
/// simplest attack against a comparison written with `starts_with`, and it
/// works: "tauri.localhost.evil.com" does start with "tauri.localhost".
#[test]
fn a_host_starting_with_the_right_name_is_refused() {
    for raw in [
        "http://tauri.localhost.evil.invalid/",
        // A subdomain, not a suffix: "localhost.evil.invalid" belongs to
        // the player's machine no more than the previous one.
        "http://localhost.evil.invalid:1420/",
        "http://localhost:14200/",
        "https://localhost:1420/",
    ] {
        assert!(!allowed(&url(raw)), "wrongly accepted: {raw}");
    }
}

/// Schemes that don't lead to a page: a `file://` would step outside the
/// asset protocol, a `javascript:` would run code in the privileged origin,
/// and a `data:` would load an arbitrary document there.
#[test]
fn dangerous_schemes_are_refused() {
    for raw in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "about:blank",
    ] {
        assert!(!allowed(&url(raw)), "wrongly accepted: {raw}");
    }
}

/// The port matters. `http://localhost` with no port isn't the dev server:
/// it's whatever service happens to listen on 80 on the player's machine.
#[test]
fn localhost_without_the_right_port_is_refused() {
    assert!(!allowed(&url("http://localhost/")));
    assert!(!allowed(&url("http://localhost:4200/")));
    assert!(!allowed(&url("http://127.0.0.1:1420/")));
}
