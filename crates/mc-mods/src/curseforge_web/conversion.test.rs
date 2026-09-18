use super::{Channel, channel_of, compatible, download_url};

fn versions(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

#[test]
fn compatibility_read_from_game_versions() {
    // Real shape returned by the site for JEI.
    let jei = versions(&["1.21", "Client", "1.21.1", "NeoForge", "Server"]);
    assert!(compatible(&jei, "1.21.1", "neoforge"));
    assert!(!compatible(&jei, "1.21.1", "fabric"));
    assert!(!compatible(&jei, "1.20.1", "neoforge"));
}

#[test]
fn a_file_without_a_declared_loader_is_accepted() {
    // Mods published before CurseForge started tagging the loader: excluding
    // them would make them missing.
    let old = versions(&["1.21.1", "Client"]);
    assert!(compatible(&old, "1.21.1", "neoforge"));
}

#[test]
fn the_right_loader_is_required_when_declared() {
    let fabric = versions(&["1.21.1", "Fabric"]);
    assert!(!compatible(&fabric, "1.21.1", "neoforge"));
    assert!(compatible(&fabric, "1.21.1", "fabric"));
}

#[test]
fn the_url_goes_through_the_site_route() {
    // And not through a reconstructed CDN URL, which would bypass the
    // author's possible refusal.
    let url = download_url(crate::curseforge_web::WEB, 238222, 8886909);
    assert!(url.starts_with("https://www.curseforge.com/api/v1/"));
    assert!(url.ends_with("/mods/238222/files/8886909/download"));
    assert!(!url.contains("forgecdn"));
}

#[test]
fn channels() {
    assert_eq!(channel_of(1), Channel::Release);
    assert_eq!(channel_of(2), Channel::Beta);
    assert_eq!(channel_of(3), Channel::Alpha);
}
