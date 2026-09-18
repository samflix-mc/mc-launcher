//! Reading and writing the settings, and what the screen allows.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::Error;

/// What the player's screen allows, so the page offers sizes that fit on
/// it.
// No `Eq`: the scale factor is a float, and equality is precisely what we
// don't want to derive here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    /// The WORK AREA, desktop panels deducted — and not the raw size.
    ///
    /// That's what "maximized" means: a window the size of the screen would
    /// go under the taskbar, and the player would never see the bottom of
    /// their inventory.
    pub width: u32,
    pub height: u32,
    /// The scale factor. Without it, offering "1920×1080" on a HiDPI
    /// screen would give a window twice too small.
    pub scale: f64,
}

/// What the player's screen allows.
///
/// `current_monitor` and not `primary_monitor`: under Wayland, the notion
/// of a primary screen doesn't always exist, and what matters is the
/// screen the window is on. Both failures are treated the same way — no
/// computed size is then offered, and the page falls back to its fixed
/// list.
#[tauri::command]
pub fn screen(app: AppHandle) -> Option<Screen> {
    let window = app.get_webview_window("main")?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;

    let area = monitor.work_area();
    Some(Screen {
        width: area.size.width,
        height: area.size.height,
        scale: monitor.scale_factor(),
    })
}

/// The settings currently in effect.
#[tauri::command]
pub fn settings() -> mc_settings::Settings {
    mc_settings::load(&mc_settings::path())
}

/// Saves the settings, and returns WHAT WAS ACTUALLY WRITTEN.
///
/// The distinction matters: if a value was brought back within bounds, the
/// window must show it right away. Returning the input would leave a
/// slider at a position the file doesn't carry, and the player would
/// believe they'd set 200 where the game will receive 32.
#[tauri::command]
pub fn save_settings(settings: mc_settings::Settings) -> Result<mc_settings::Settings, Error> {
    Ok(mc_settings::save(&mc_settings::path(), &settings)?)
}

/// The folders offered for opening from the "Advanced" section.
///
/// A closed enum and NOT a path: a command that accepted a path from the
/// front would allow opening anything on the machine, from a page whose
/// content partly comes from a remote host.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Folder {
    Data,
    Config,
    Logs,
    Instance,
}

/// Opens one of the launcher's folders in the system's file explorer.
///
/// Requires the `opener:allow-open-path` permission, which does NOT appear
/// in `core:default`: without it, the call fails with an ACL refusal that
/// only shows up in a packaged build — never in `tauri dev`.
#[tauri::command]
pub fn open_folder(app: AppHandle, folder: Folder) -> Result<(), Error> {
    use tauri_plugin_opener::OpenerExt;

    let locations = mc_paths::current();
    let path = match folder {
        Folder::Data => locations.data.clone(),
        Folder::Config => locations.config.clone(),
        Folder::Logs => locations.logs.clone(),
        Folder::Instance => {
            let options = mc_pack::Options::default();
            options
                .layout
                .instance(mc_pack::state::DEFAULT_NAME)
                .game_dir
        }
    };

    // Created if needed: opening a folder that doesn't exist yet would open
    // an empty explorer window, or nothing at all depending on the system.
    if let Err(error) = std::fs::create_dir_all(&path) {
        tracing::warn!(error = %error, path = %path.display(), "folder not created");
    }

    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|error| Error(format!("folder not opened: {error}")))
}
