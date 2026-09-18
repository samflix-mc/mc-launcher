//! Read and write the settings file.

use std::path::{Path, PathBuf};

use crate::types::Settings;

/// Where the settings live.
pub fn path() -> PathBuf {
    mc_paths::current().config.join("settings.json")
}

/// Re-reads the settings, or returns the defaults.
///
/// **Never returns an error.** A file that's missing, unreadable, or of an
/// unknown schema all give the defaults — losing your settings is annoying,
/// not being able to open the launcher is worse. The cases are told apart
/// in the log, not in the return type.
///
/// What gets re-read is ALWAYS validated: the file can be hand-edited, and
/// that's exactly what a player chasing frames per second will do.
pub fn load(path: &Path) -> Settings {
    let Ok(raw) = std::fs::read(path) else {
        tracing::debug!(file = %path.display(), "no settings saved, defaults");
        return Settings::default();
    };

    let mut settings: Settings = match serde_json::from_slice(&raw) {
        Ok(settings) => settings,
        Err(error) => {
            tracing::warn!(
                error = %error,
                file = %path.display(),
                "unreadable settings: falling back to defaults"
            );
            return Settings::default();
        }
    };

    if settings.schema != crate::types::SCHEMA {
        // Not a refusal: `#[serde(default)]` on each section means the
        // fields from a neighboring schema still get re-read, and the ones
        // we don't know about fall back to their default. That beats losing
        // everything.
        tracing::info!(
            found = settings.schema,
            expected = crate::types::SCHEMA,
            "settings from another schema: whatever re-reads is kept"
        );
    }

    settings.validate();
    settings
}

/// Writes the settings, after validating them.
///
/// Validation happens here and not in the caller: it must not be possible
/// to forget it, and this is the only place a setting enters disk.
pub fn save(path: &Path, settings: &Settings) -> anyhow::Result<Settings> {
    let mut valid = settings.clone();
    valid.validate();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(&valid)?;
    json.push('\n');
    mc_dl::write_atomic(path, json.as_bytes())?;

    // We return what was WRITTEN, not what we received: if a value got
    // clamped into bounds, the window must show it right away. Returning
    // the input would leave a slider at a position the file doesn't carry,
    // until the next reload.
    Ok(valid)
}

#[cfg(test)]
#[path = "reading.test.rs"]
mod tests;
