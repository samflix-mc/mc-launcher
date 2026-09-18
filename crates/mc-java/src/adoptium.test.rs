use super::{API, platform, url_assets};

#[test]
fn the_url_requests_the_latest_binary_for_the_platform() {
    assert_eq!(
        url_assets(API, 21, "linux", "x64", "jre"),
        "https://api.adoptium.net/v3/assets/latest/21/hotspot\
         ?architecture=x64&image_type=jre&os=linux&vendor=eclipse"
    );
}

/// Adoptium's vocabulary isn't Rust's: `macos` is called `mac` there,
/// `x86_64` is called `x64`. Getting it wrong gives an empty list, hence
/// "Adoptium doesn't publish Java 21" on a perfectly ordinary machine.
#[test]
fn the_current_platform_has_a_name_at_adoptium() {
    let (os, arch) = platform().expect("this machine is covered by Temurin");
    assert!(
        ["linux", "mac", "windows"].contains(&os),
        "unexpected system: {os}"
    );
    assert!(
        ["x64", "aarch64"].contains(&arch),
        "unexpected architecture: {arch}"
    );
}
