//! What the player configures, and the bounds of each setting.

use serde::{Deserialize, Serialize};

/// Format version. A file from another version is re-read with the defaults
/// rather than failing the launch: losing your settings is annoying, not
/// being able to play is worse.
pub const SCHEMA: u32 = 1;

/// Everything the player can change.
///
/// Four sections, and their split isn't cosmetic: it says WHO reads each
/// value.
///
/// - [`Game`] — keys merged into `options.txt`. Minecraft reads them, not us.
/// - [`Window`] — the GAME's window, passed as launch arguments.
/// - [`Launcher`] — what the launcher does on its own at launch.
/// - [`Appearance`] — the LAUNCHER's window. The game knows nothing about it.
///
/// Mixing up `window` and `appearance` is the mistake made every time: the
/// first is the game's, the second is ours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema: u32,
    pub game: Game,
    pub window: Window,
    pub launcher: Launcher,
    pub appearance: Appearance,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            game: Game::default(),
            window: Window::default(),
            launcher: Launcher::default(),
            appearance: Appearance::default(),
        }
    }
}

/// The keys the launcher merges into `options.txt`.
///
/// Only the ones a player actually adjusts and a modpack doesn't drive.
/// `graphicsMode` is excluded on purpose: shaders replace it, and forcing it
/// from the launcher would undo what Iris set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Game {
    /// Render distance, in chunks. 2 to 32.
    pub render_distance: u8,
    /// Simulation distance, in chunks. 5 to 32 — the floor is the game's own,
    /// below which entities stop updating around the player.
    pub simulation_distance: u8,
    /// Frame rate cap. 10 to 260 — above 260, Minecraft writes `max` and
    /// stops limiting.
    pub max_fps: u16,
    /// UI scale. 0 means "automatic"; otherwise 1 or more.
    pub gui_scale: u8,
    /// Vertical sync.
    pub vsync: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            render_distance: 12,
            simulation_distance: 10,
            max_fps: 120,
            gui_scale: 0,
            vsync: true,
        }
    }
}

/// How the GAME's window opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowMode {
    /// At the requested size.
    Windowed,
    /// At the screen's work area, desktop panels excluded.
    Maximized,
    /// Exclusive fullscreen.
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Window {
    pub mode: WindowMode,
    /// The requested size in windowed mode. Ignored in the other two.
    pub width: u32,
    pub height: u32,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            mode: WindowMode::Windowed,
            width: 1280,
            height: 720,
        }
    }
}

impl Window {
    /// The value of `options.txt`'s `fullscreen` key.
    ///
    /// DERIVED from the mode, not a separate field. That's what keeps the
    /// two from diverging — and they would: F11 toggles this key
    /// mid-session, and the game persists it. A `fullscreen` field
    /// independent of the mode would have made fullscreen a one-way switch,
    /// where pressing it from the game would be undone on the next launch.
    pub fn fullscreen(&self) -> bool {
        self.mode == WindowMode::Fullscreen
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Launcher {
    /// Memory allocated to the JVM, in megabytes.
    ///
    /// `None` lets the JVM decide — a quarter of the machine's memory, which
    /// isn't enough for a modpack and yields an `OutOfMemoryError` after
    /// twenty minutes. The launcher's default is therefore NOT `None`.
    pub memory_mb: Option<u32>,
    /// Minimize the launcher's window when the game starts.
    pub minimize_on_launch: bool,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            memory_mb: Some(4096),
            minimize_on_launch: true,
        }
    }
}

/// The LAUNCHER's window backdrop.
///
/// An enumerated identifier and NOT a path. A path would make the front end
/// the owner of the list, and force Rust to persist a value it can't
/// validate — a path to a deleted file, to a directory, to outside the
/// launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backdrop {
    Spawn,
    Nether,
    End,
    Plain,
}

impl Backdrop {
    /// The file name, under `public/fonds/`.
    pub fn file(self) -> &'static str {
        match self {
            Backdrop::Spawn => "spawn.webp",
            Backdrop::Nether => "nether.webp",
            Backdrop::End => "end.webp",
            // No image at all: a gradient, for whoever finds photos noisy or
            // has a slow screen.
            Backdrop::Plain => "",
        }
    }

    pub const ALL: [Backdrop; 4] = [
        Backdrop::Spawn,
        Backdrop::Nether,
        Backdrop::End,
        Backdrop::Plain,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub backdrop: Backdrop,
    /// The opacity of the scrim laid between the image and the interface.
    ///
    /// **Bounded on the low end, and it's the only bound that isn't a
    /// comfort.** Below the floor, the interface text no longer holds the
    /// minimum contrast against the lightest bundled image: labels become
    /// unreadable on only part of the screen, which is the worst way to
    /// break an interface — it looks like a rendering defect, not a
    /// setting.
    ///
    /// The slider therefore runs from the floor to 1, not from 0 to 1.
    pub scrim: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            backdrop: Backdrop::Spawn,
            // Enough to lay the interface over the image without erasing
            // it. Contrast no longer depends on this value — see
            // `SCRIM_FLOOR`.
            scrim: 0.3,
        }
    }
}

#[cfg(test)]
#[path = "types.test.rs"]
mod tests;
