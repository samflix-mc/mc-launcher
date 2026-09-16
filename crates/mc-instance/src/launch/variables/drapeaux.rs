//! Les drapeaux actifs, déduits des options demandées.

use crate::vanilla::Features;

use crate::launch::session::{LaunchOptions, QuickPlay};

/// Une règle conditionnée à un drapeau ne s'applique que si le launcher l'a
/// activé : c'est ce qui fait que `--quickPlayMultiplayer` n'apparaît sur la
/// ligne de commande que lorsqu'on demande effectivement de rejoindre un
/// serveur.
pub(in crate::launch) fn active_features(options: &LaunchOptions) -> Features {
    let mut features = Features::new();
    if options.resolution.is_some() {
        features.insert("has_custom_resolution".into());
    }
    if options.quick_play.is_some() {
        // `has_quick_plays_support` conditionne `--quickPlayPath`, le journal
        // que le jeu écrit ; les deux autres désignent la destination.
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
