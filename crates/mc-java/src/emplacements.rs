//! Où chercher un Java, et où le launcher range le sien.

use std::path::{Path, PathBuf};

/// Répertoire où le launcher installe ses runtimes Java.
pub fn default_runtime_dir() -> PathBuf {
    mc_dl::data_dir().join("runtime")
}

/// Emplacement du runtime géré pour une version majeure donnée.
pub fn managed_home(runtime_dir: &Path, major: u32) -> PathBuf {
    runtime_dir.join(format!("temurin-{major}"))
}

pub(crate) fn java_exe(home: &Path) -> PathBuf {
    if cfg!(windows) {
        home.join("bin").join("java.exe")
    } else {
        home.join("bin").join("java")
    }
}

/// Candidats à tester, du plus fiable au plus douteux.
///
/// Le runtime géré passe en premier : s'il est là, c'est le launcher qui l'a
/// installé et vérifié, inutile de sonder le système.
pub(crate) fn candidates(runtime_dir: &Path, major: u32) -> Vec<PathBuf> {
    let mut found = vec![java_exe(&managed_home(runtime_dir, major))];

    if let Some(home) = std::env::var_os("JAVA_HOME") {
        found.push(java_exe(Path::new(&home)));
    }

    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let exe = dir.join(if cfg!(windows) { "java.exe" } else { "java" });
            if exe.is_file() {
                found.push(exe);
            }
        }
    }

    // Emplacements usuels des paquets système. Les distributions installent
    // plusieurs JDK côte à côte et n'en exposent qu'un dans le `PATH` ; le 21
    // demandé est souvent présent mais non par défaut.
    let roots: &[&str] = if cfg!(target_os = "macos") {
        &["/Library/Java/JavaVirtualMachines"]
    } else if cfg!(windows) {
        &[
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
        ]
    } else {
        &["/usr/lib/jvm", "/usr/lib64/jvm", "/opt/java"]
    };
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let home = entry.path();
            // macOS empaquette le JDK dans un bundle.
            found.push(java_exe(&home.join("Contents").join("Home")));
            found.push(java_exe(&home));
        }
    }

    found
}

#[cfg(test)]
#[path = "emplacements.test.rs"]
mod tests;
