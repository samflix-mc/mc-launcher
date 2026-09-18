//! Where the launcher keeps what it installs, what it configures, and what
//! it discards.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The name under which the launcher appears in the system's directories.
///
/// **The application's identifier, not the network's name.** It's exactly
/// what Tauri's `app_data_dir()` and `app_config_dir()` compose: the two
/// halves of the program — the window, which queries the resolver, and the
/// command line, which derives it from the environment — therefore end up at
/// the same directory by two different paths.
///
/// This segment was `samflix-mc` until September 18, 2026, and the argument
/// was sound: don't abandon what was already laid down. It was deliberately
/// reversed, for a stronger reason — a launcher that keeps its things
/// somewhere other than where its own framework expects them is a trap that
/// gets rediscovered on every reading of the code, and the doubt comes back
/// every time. The cost of the reversal is paid ONCE, by moving a directory;
/// the cost of the doubt is paid on every pass.
///
/// **What did NOT change**: the keyring service is still named `samflix-mc`.
/// It's not a path, it's a key in the secrets manager — renaming it would
/// disconnect open sessions without gaining anything.
pub const SEGMENT: &str = "mc.samflix.launcher";

/// The four roots the launcher needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locations {
    /// What's big and rebuildable: instances, Java runtimes, caches.
    pub data: PathBuf,
    /// What's small and precious: the session, the settings.
    pub config: PathBuf,
    /// Today's logs and previous days'.
    pub logs: PathBuf,
    /// What doesn't survive a restart: writes in progress.
    pub temporary: PathBuf,
}

/// The raw roots, before the launcher's segment is appended to them.
///
/// Separated from [`Locations`] so that the DERIVATION — which cannot
/// diverge from one platform to another — is distinct from the CHOICE of
/// roots, which necessarily does diverge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bases {
    pub data: PathBuf,
    pub config: PathBuf,
    pub temporary: PathBuf,
}

/// What's derived from the roots, and nothing else.
///
/// Nothing but `join`s. It's the only part that can be said not to vary by
/// platform, and that's why it's kept separate: the application's
/// `paths.rs` compares what Tauri gives it to what [`from_system`] finds,
/// and that comparison would make no sense if the derivation itself could
/// differ.
pub fn from_bases(bases: Bases) -> Locations {
    Locations {
        // `<data>/logs`, and NOT the system's "app_log_dir". Under macOS,
        // that one stores under `~/Library/Logs`, i.e. OUTSIDE the data:
        // deleting the data directory would no longer be enough to start
        // fresh, which is the only promise this tree makes.
        logs: bases.data.join("logs"),
        data: bases.data,
        config: bases.config,
        temporary: bases.temporary,
    }
}

/// What the system says, platform by platform.
///
/// The function picks the branch; the three functions it calls are compiled
/// EVERYWHERE and carry no `cfg`. That's what makes it possible to exercise
/// the Windows branch from a Linux runner: without this, two branches out of
/// three would never be run by CI, and each would produce surviving mutants
/// indefinitely.
pub fn from_system() -> Locations {
    let read = |key: &str| std::env::var_os(key);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let temp = std::env::temp_dir();

    let bases = if cfg!(windows) {
        bases_windows(&read, temp)
    } else if cfg!(target_os = "macos") {
        bases_macos(&read, home, temp)
    } else {
        bases_linux(&read, home, temp)
    };

    from_bases(bases)
}

/// The XDG convention, and its two traps.
///
/// A variable that's SET BUT EMPTY designates nothing: treating it as a root
/// would put the data at `/samflix-mc`, at the disk's root. And a missing
/// `HOME` — a service, a container — must not cause a panic: it falls back
/// to the current directory, which is bad but visible, whereas a panic would
/// be silent in a graphical application.
pub fn bases_linux(
    read: &impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
    temporary: PathBuf,
) -> Bases {
    let root = |variable: &str, default: &[&str]| -> PathBuf {
        match read(variable).filter(|value| !value.is_empty()) {
            Some(value) => PathBuf::from(value).join(SEGMENT),
            None => default
                .iter()
                .fold(current_or(home.clone()), |path, part| path.join(part))
                .join(SEGMENT),
        }
    };

    Bases {
        data: root("XDG_DATA_HOME", &[".local", "share"]),
        // The configuration is NOT under the data, and this is the only
        // platform where the distinction actually exists: it's what makes it
        // possible to delete eight hundred megabytes of instances without
        // losing the session or the settings.
        config: root("XDG_CONFIG_HOME", &[".config"]),
        temporary: temporary.join(SEGMENT),
    }
}

/// macOS puts everything under `Application Support`.
///
/// Data and configuration at the SAME place, and that's not a shortcut: it's
/// what Tauri's resolver does (`path/desktop.rs:62-63,73-74`), and departing
/// from it would make the application diverge from its own crates. The
/// consequence belongs in the documentation rather than a fix here: the
/// promise "delete the data without losing the preferences" only holds under
/// Linux.
pub fn bases_macos(
    read: &impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
    temporary: PathBuf,
) -> Bases {
    let _ = read;
    let support = current_or(home)
        .join("Library")
        .join("Application Support")
        .join(SEGMENT);

    Bases {
        data: support.clone(),
        config: support,
        temporary: temporary.join(SEGMENT),
    }
}

/// Windows puts everything under `APPDATA`, the roaming one.
///
/// `APPDATA` and not `LOCALAPPDATA`: that's what Tauri's resolver returns for
/// both, and a roaming profile therefore carries the instances along with
/// it. That's debatable for eight hundred megabytes; it's not this crate's
/// place to decide alone, since the application will follow Tauri anyway —
/// and two different locations would be far worse than one bad one.
pub fn bases_windows(read: &impl Fn(&str) -> Option<OsString>, temporary: PathBuf) -> Bases {
    let base = match read("APPDATA").filter(|value| !value.is_empty()) {
        Some(value) => PathBuf::from(value).join(SEGMENT),
        // Without APPDATA there's no user profile: the current directory is
        // the only fallback that asks nothing of anyone.
        None => PathBuf::from(".").join(SEGMENT),
    };

    Bases {
        data: base.clone(),
        config: base,
        temporary: temporary.join(SEGMENT),
    }
}

/// The home directory, or the current directory otherwise.
fn current_or(home: Option<PathBuf>) -> PathBuf {
    home.unwrap_or_else(|| PathBuf::from("."))
}

impl Locations {
    /// Creates the four directories if they don't exist.
    ///
    /// Called at startup rather than on the first write: a player whose disk
    /// is full must learn it before having downloaded four hundred
    /// megabytes, not after.
    pub fn create(&self) -> std::io::Result<()> {
        for path in [&self.data, &self.config, &self.logs, &self.temporary] {
            std::fs::create_dir_all(path)?;
        }
        Ok(())
    }

    /// The four roots, named, for diagnostics.
    pub fn list(&self) -> [(&'static str, &Path); 4] {
        [
            ("data", self.data.as_path()),
            ("config", self.config.as_path()),
            ("logs", self.logs.as_path()),
            ("temporary", self.temporary.as_path()),
        ]
    }
}

#[cfg(test)]
#[path = "locations.test.rs"]
mod tests;
