//! Où le launcher range ce qu'il installe.

use std::path::PathBuf;

/// Racine des données du launcher, selon la convention de chaque système.
///
/// Tout ce que le launcher installe y vit : runtimes Java, instances, caches.
/// Un seul endroit à supprimer pour repartir de zéro, et rien qui traîne dans
/// le répertoire personnel.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);

    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("samflix-mc");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home.clone() {
            return home
                .join("Library")
                .join("Application Support")
                .join("samflix-mc");
        }
    }

    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(xdg).join("samflix-mc");
    }
    home.unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("samflix-mc")
}

#[cfg(test)]
#[path = "emplacements.test.rs"]
mod tests;
