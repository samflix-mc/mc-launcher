//! Ce que ${…} désigne dans les arguments du descripteur.

mod drapeaux;

use std::collections::BTreeMap;
use std::path::Path;

use super::session::{LaunchOptions, QuickPlay, Session};

pub(in crate::launch) use drapeaux::active_features;

#[allow(clippy::too_many_arguments)]
pub(super) fn variables(
    version_id: &str,
    base_id: &str,
    game_dir: &Path,
    shared: &Path,
    assets_index: &str,
    natives: &Path,
    libraries: &Path,
    classpath: &str,
    separator: &str,
    session: &Session,
    options: &LaunchOptions,
) -> BTreeMap<String, String> {
    let mut v = BTreeMap::new();
    let mut set = |k: &str, value: String| {
        v.insert(k.to_string(), value);
    };

    set("auth_player_name", session.name.clone());
    set("auth_uuid", session.uuid.clone());
    set("auth_access_token", session.token.clone());
    set("auth_xuid", session.xuid.clone());
    set("clientid", session.client_id.clone());
    set("user_type", session.user_type.clone());
    set("auth_session", format!("token:{}", session.token));

    set("version_name", version_id.to_string());
    set("version_type", "release".into());
    set("game_directory", game_dir.display().to_string());
    set("assets_root", shared.join("assets").display().to_string());
    set("game_assets", shared.join("assets").display().to_string());
    set("assets_index_name", assets_index.to_string());
    set("natives_directory", natives.display().to_string());
    set("library_directory", libraries.display().to_string());
    set("classpath", classpath.to_string());
    set("classpath_separator", separator.to_string());
    set("launcher_name", "Helm".into());
    set("launcher_version", env!("CARGO_PKG_VERSION").to_string());
    // Le jar du socle, que NeoForge nomme dans son `ignoreList`.
    set("primary_jar_name", format!("{base_id}.jar"));

    if let Some((width, height)) = options.resolution {
        set("resolution_width", width.to_string());
        set("resolution_height", height.to_string());
    }
    match &options.quick_play {
        Some(QuickPlay::Multiplayer(target)) => {
            set("quickPlayMultiplayer", target.clone());
            set(
                "quickPlayPath",
                game_dir.join("quickPlay.json").display().to_string(),
            );
        }
        Some(QuickPlay::Singleplayer(world)) => {
            set("quickPlaySingleplayer", world.clone());
            set(
                "quickPlayPath",
                game_dir.join("quickPlay.json").display().to_string(),
            );
        }
        None => {}
    }
    v
}

#[cfg(test)]
#[path = "variables.test.rs"]
mod tests;
