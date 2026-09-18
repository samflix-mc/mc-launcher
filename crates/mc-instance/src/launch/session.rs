//! Who's playing, and under what conditions.

/// Player identity passed to the game.
///
/// Deliberately independent from `mc-auth`: an offline session and a
/// Microsoft session produce the same structure, and this module doesn't
/// need to know which one it's serving.
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub uuid: String,
    /// Access token. Empty when offline — the game accepts that and then
    /// only joins servers running `online-mode=false`.
    pub token: String,
    /// `msa` for a Microsoft account, `legacy` otherwise.
    pub user_type: String,
    pub xuid: String,
    pub client_id: String,
}

impl Session {
    /// Offline session, for a server running `online-mode=false`.
    pub fn offline(name: impl Into<String>, uuid: impl Into<String>) -> Session {
        Session {
            name: name.into(),
            uuid: uuid.into(),
            // The game requires the argument but doesn't validate it
            // offline. The value `"0"` is the one common launchers use: an
            // empty string would break argument parsing.
            token: "0".into(),
            user_type: "legacy".into(),
            xuid: String::new(),
            client_id: String::new(),
        }
    }

    /// Microsoft session, for an online server.
    ///
    /// `xuid` and `client_id` stay empty: neither Prism, PolyMC, nor
    /// OpenLauncher pass `--xuid` or `--clientId` to the game, and the
    /// server checks identity with Mojang from the token alone.
    pub fn online(
        name: impl Into<String>,
        uuid: impl Into<String>,
        token: impl Into<String>,
    ) -> Session {
        Session {
            name: name.into(),
            uuid: uuid.into(),
            token: token.into(),
            user_type: "msa".into(),
            xuid: String::new(),
            client_id: String::new(),
        }
    }
}

/// Session to join directly at startup.
#[derive(Debug, Clone)]
pub enum QuickPlay {
    /// Server, in `host` or `host:port` form.
    Multiplayer(String),
    /// Local world, by its folder name.
    Singleplayer(String),
}

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Maximum JVM memory, in mebibytes.
    pub memory_mb: Option<u32>,
    pub quick_play: Option<QuickPlay>,
    pub resolution: Option<(u32, u32)>,
    /// Open the game in fullscreen.
    ///
    /// ## A flag, not a value
    ///
    /// `--fullscreen` is a flag with NO value: writing `--fullscreen true`
    /// would make the game take "true" for the name of a world to open. So
    /// it's either added or not, never with an argument.
    ///
    /// ## The double-source trap
    ///
    /// `fullscreen` is ALSO an `options.txt` key, which F11 toggles during
    /// a session and which the game persists. Driving only the command-line
    /// argument would turn fullscreen into a one-way switch: the player
    /// would leave it with F11, and find it back on at the next launch
    /// without understanding why.
    ///
    /// The two are therefore written together — see `mc_settings::fusionner`.
    pub fullscreen: bool,
    /// JVM arguments added before those from the descriptor.
    pub extra_jvm: Vec<String>,
}

#[cfg(test)]
#[path = "session.test.rs"]
mod tests;
