//! Where to look for a Java, and where the launcher keeps its own.

use std::path::{Path, PathBuf};

/// Directory where the launcher installs its Java runtimes.
pub fn default_runtime_dir() -> PathBuf {
    mc_paths::current().data.join("runtime")
}

/// Location of the managed runtime for a given major version.
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

/// Candidates to test, from most trustworthy to most dubious.
///
/// The managed runtime comes first: if it's there, the launcher installed
/// and verified it, no need to probe the system.
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

    // Usual locations of system packages. Distributions install several JDKs
    // side by side and expose only one on `PATH`; the requested 21 is often
    // present but not the default.
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
            // macOS packages the JDK inside a bundle.
            found.push(java_exe(&home.join("Contents").join("Home")));
            found.push(java_exe(&home));
        }
    }

    found
}

#[cfg(test)]
#[path = "locations.test.rs"]
mod tests;
