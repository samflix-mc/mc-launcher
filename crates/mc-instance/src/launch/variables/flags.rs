//! The active flags, derived from the requested options.

use crate::vanilla::Features;

use crate::launch::session::{LaunchOptions, QuickPlay};

/// A rule conditioned on a flag only applies if the launcher activated it:
/// that's what makes `--quickPlayMultiplayer` appear on the command line
/// only when actually asked to join a server.
pub(in crate::launch) fn active_features(options: &LaunchOptions) -> Features {
    let mut features = Features::new();
    if options.resolution.is_some() {
        features.insert("has_custom_resolution".into());
    }
    if options.quick_play.is_some() {
        // `has_quick_plays_support` gates `--quickPlayPath`, the log the
        // game writes; the other two name the destination.
        features.insert("has_quick_plays_support".into());
        match options.quick_play {
            Some(QuickPlay::Multiplayer(_)) => {
                features.insert("is_quick_play_multiplayer".into());
            }
            Some(QuickPlay::Singleplayer(_)) => {
                features.insert("is_quick_play_singleplayer".into());
            }
            None => {}
        }
    }
    features
}
