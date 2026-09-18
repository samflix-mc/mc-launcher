use super::Backdrop;

/// Each backdrop names ITS file, and "plain" has none.
///
/// Nothing checked this: rendering an empty string for every backdrop — or
/// any string at all — would leave the suite green. The symptom would be a
/// launcher showing the same backdrop no matter the setting, or none.
#[test]
fn each_backdrop_names_its_file() {
    assert_eq!(Backdrop::Spawn.file(), "spawn.webp");
    assert_eq!(Backdrop::Nether.file(), "nether.webp");
    assert_eq!(Backdrop::End.file(), "end.webp");
    // "plain" is a gradient declared in CSS: no image, and it's the only
    // legitimate empty value.
    assert_eq!(Backdrop::Plain.file(), "");
}

/// And no two backdrops share a file.
#[test]
fn no_two_backdrops_share_their_image() {
    let with_image: Vec<&str> = Backdrop::ALL
        .iter()
        .map(|backdrop| backdrop.file())
        .filter(|name| !name.is_empty())
        .collect();
    let mut unique = with_image.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), with_image.len(), "{with_image:?}");
}
