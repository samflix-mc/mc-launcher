use super::super::session::QuickPlay;
use super::{LaunchOptions, active_features};

#[test]
fn les_drapeaux_suivent_les_options() {
    let sans = active_features(&LaunchOptions::default());
    assert!(sans.is_empty());

    let avec = active_features(&LaunchOptions {
        quick_play: Some(QuickPlay::Multiplayer("mc.exemple.fr".into())),
        resolution: Some((1280, 720)),
        ..Default::default()
    });
    assert!(avec.contains("is_quick_play_multiplayer"));
    assert!(avec.contains("has_quick_plays_support"));
    assert!(avec.contains("has_custom_resolution"));
    assert!(!avec.contains("is_quick_play_singleplayer"));
}
