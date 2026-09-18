use super::{latest_stable, series_for};

#[test]
fn series_derived_from_the_game_version() {
    assert_eq!(series_for("1.21.1").as_deref(), Some("21.1."));
    assert_eq!(series_for("1.21").as_deref(), Some("21.0."));
    assert_eq!(series_for("1.20.4").as_deref(), Some("20.4."));
    // NeoForge doesn't cover versions before the 1.x versioning scheme.
    assert_eq!(series_for("21w07a"), None);
}

/// Three rules decide the installed version, and each one matters.
///
/// The series: a version for a different Minecraft won't start. Betas,
/// excluded: they appear in the same list, and installing one by mistake
/// changes the game under players' feet. Sorting by patch: versions are
/// published in order, but a republish can disorder the list.
#[test]
fn the_latest_stable_version_of_the_series_is_kept() {
    let published = vec![
        "21.1.9".to_string(),
        "21.1.250".to_string(),
        "21.1.100".to_string(),
        // A beta of the same series: never installed by default.
        "21.1.300-beta".to_string(),
        // A different series: for a different game version.
        "21.4.10".to_string(),
    ];

    assert_eq!(
        latest_stable(published.clone(), "21.1."),
        Some("21.1.250".to_string()),
        "the highest patch of the series, excluding beta"
    );
    assert_eq!(
        latest_stable(published, "21.4."),
        Some("21.4.10".to_string())
    );
}

/// A series nobody has published yields nothing, and that's not a failure:
/// it's what happens on a Minecraft release day, before NeoForge catches up.
#[test]
fn a_series_with_no_published_version_yields_nothing() {
    assert_eq!(latest_stable(Vec::new(), "21.1."), None);
    assert_eq!(
        latest_stable(vec!["21.1.0-beta".to_string()], "21.1."),
        None,
        "a series with only betas has nothing stable"
    );
}
