use super::{Bases, SEGMENT, bases_linux, bases_macos, bases_windows, from_bases, from_system};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// A counterfeit environment, to exercise the three branches without
/// touching the process's own — which other tests read at the same time.
fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let table: HashMap<String, OsString> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), OsString::from(*value)))
        .collect();
    move |key: &str| table.get(key).cloned()
}

fn home() -> Option<PathBuf> {
    Some(PathBuf::from("/home/player"))
}

fn tmp() -> PathBuf {
    PathBuf::from("/tmp")
}

// --- Derivation -----------------------------------------------------------

/// The logs live UNDER the data. That's what makes deleting the data
/// directory enough to start fresh — the only promise this tree makes, and
/// the one a system `app_log_dir` would break under macOS by storing
/// elsewhere.
#[test]
fn the_logs_live_under_the_data() {
    let derived = from_bases(Bases {
        data: PathBuf::from("/d"),
        config: PathBuf::from("/c"),
        temporary: PathBuf::from("/t"),
    });

    assert_eq!(derived.logs, PathBuf::from("/d/logs"));
    assert!(derived.logs.starts_with(&derived.data));
}

/// The derivation doesn't touch anything else: what it's given comes back
/// unchanged. That's what lets the application place Tauri's roots and get
/// the same tree as the crates.
#[test]
fn the_derivation_rewrites_no_root() {
    let bases = Bases {
        data: PathBuf::from("/d"),
        config: PathBuf::from("/c"),
        temporary: PathBuf::from("/t"),
    };
    let derived = from_bases(bases.clone());

    assert_eq!(derived.data, bases.data);
    assert_eq!(derived.config, bases.config);
    assert_eq!(derived.temporary, bases.temporary);
}

// --- Linux ------------------------------------------------------------------

/// `XDG_DATA_HOME` takes priority when it's set — that's the system's
/// convention, and a machine that follows it must not end up with two
/// locations.
///
/// This test replaces `the_system_convention_is_respected` from
/// mc-dl/src/locations.test.rs, which used to write to the process's
/// environment to exercise it.
#[test]
fn linux_respects_the_system_convention() {
    let bases = bases_linux(
        &environment(&[
            ("XDG_DATA_HOME", "/elsewhere/shared"),
            ("XDG_CONFIG_HOME", "/elsewhere/settings"),
        ]),
        home(),
        tmp(),
    );

    assert_eq!(
        bases.data,
        PathBuf::from(format!("/elsewhere/shared/{SEGMENT}"))
    );
    assert_eq!(
        bases.config,
        PathBuf::from(format!("/elsewhere/settings/{SEGMENT}"))
    );
}

/// A variable that's SET BUT EMPTY designates nothing. Treating it as a root
/// would put the data at `/samflix-mc`, at the disk's root — where a player
/// isn't allowed to write, and where no one would think to look.
#[test]
fn linux_ignores_an_empty_variable() {
    let bases = bases_linux(
        &environment(&[("XDG_DATA_HOME", ""), ("XDG_CONFIG_HOME", "")]),
        home(),
        tmp(),
    );

    assert_eq!(
        bases.data,
        PathBuf::from(format!("/home/player/.local/share/{SEGMENT}"))
    );
    assert_eq!(
        bases.config,
        PathBuf::from(format!("/home/player/.config/{SEGMENT}"))
    );
}

/// Without a variable, the XDG spec's defaults.
#[test]
fn linux_falls_back_to_the_xdg_defaults() {
    let bases = bases_linux(&environment(&[]), home(), tmp());

    assert_eq!(
        bases.data,
        PathBuf::from(format!("/home/player/.local/share/{SEGMENT}"))
    );
    assert_eq!(
        bases.config,
        PathBuf::from(format!("/home/player/.config/{SEGMENT}"))
    );
}

/// Without `HOME` — a service, a container — we fall back to the current
/// directory. That's bad, but visible: a panic would be silent in a
/// graphical application, which swallows its error output.
#[test]
fn linux_without_home_does_not_panic() {
    let bases = bases_linux(&environment(&[]), None, tmp());

    assert_eq!(
        bases.data,
        PathBuf::from(format!("./.local/share/{SEGMENT}")),
        "{:?}",
        bases.data
    );
}

/// The separation that justifies all the rest: under Linux, and only there,
/// you can delete eight hundred megabytes of instances without losing the
/// session.
#[test]
fn linux_separates_the_data_from_the_configuration() {
    let bases = bases_linux(&environment(&[]), home(), tmp());
    assert_ne!(bases.data, bases.config);
    assert!(!bases.config.starts_with(&bases.data));
}

// --- macOS --------------------------------------------------------------------

/// `Application Support`, with both roots merged — what Tauri's resolver
/// does. Departing from it would make the application diverge from its own
/// crates, which is worse than a questionable location.
#[test]
fn macos_puts_everything_under_application_support() {
    let bases = bases_macos(&environment(&[]), home(), tmp());

    assert_eq!(
        bases.data,
        PathBuf::from(format!(
            "/home/player/Library/Application Support/{SEGMENT}"
        ))
    );
    assert_eq!(bases.config, bases.data);
}

/// And the consequence, written here so it isn't rediscovered: the promise
/// "delete the data without losing the preferences" does NOT hold here.
#[test]
fn macos_does_not_separate_the_data_from_the_configuration() {
    let bases = bases_macos(&environment(&[]), home(), tmp());
    assert_eq!(bases.config, bases.data);
}

#[test]
fn macos_without_home_does_not_panic() {
    let bases = bases_macos(&environment(&[]), None, tmp());
    assert!(bases.data.ends_with(SEGMENT), "{:?}", bases.data);
}

// --- Windows --------------------------------------------------------------------

#[test]
fn windows_puts_everything_under_appdata() {
    let bases = bases_windows(
        &environment(&[("APPDATA", r"C:\Users\player\AppData\Roaming")]),
        tmp(),
    );

    assert_eq!(
        bases.data,
        PathBuf::from(r"C:\Users\player\AppData\Roaming").join(SEGMENT)
    );
    assert_eq!(bases.config, bases.data);
}

/// An empty `APPDATA` is treated as absent: otherwise the disk's root.
#[test]
fn windows_ignores_an_empty_appdata() {
    let bases = bases_windows(&environment(&[("APPDATA", "")]), tmp());
    assert_eq!(bases.data, PathBuf::from(".").join(SEGMENT));
}

#[test]
fn windows_without_appdata_does_not_panic() {
    let bases = bases_windows(&environment(&[]), tmp());
    assert_eq!(bases.data, PathBuf::from(".").join(SEGMENT));
}

// --- The temporary directory, common to all three ------------------------------

/// The temporary directory carries the segment on all three platforms:
/// without it, two applications writing a file with the same name to `/tmp`
/// would step on each other.
#[test]
fn the_temporary_directory_carries_the_segment_everywhere() {
    for base in [
        bases_linux(&environment(&[]), home(), tmp()),
        bases_macos(&environment(&[]), home(), tmp()),
        bases_windows(&environment(&[]), tmp()),
    ] {
        assert_eq!(base.temporary, PathBuf::from("/tmp").join(SEGMENT));
    }
}

// --- The whole --------------------------------------------------------------

/// Everything the launcher installs lives under a single named root: one
/// place to delete to start fresh, and nothing left lying around in the home
/// directory.
///
/// Picks up `the_data_lives_under_a_single_named_directory` from mc-dl.
#[test]
fn the_data_lives_under_a_single_named_directory() {
    let e = from_system();
    assert!(e.data.ends_with(SEGMENT), "{:?}", e.data);
    assert!(e.data.is_absolute() || e.data.starts_with("."));
    assert!(e.logs.starts_with(&e.data), "{:?}", e.logs);
}

/// `create` creates ALL FOUR directories, not just three.
///
/// Nothing checked it: the function could do nothing at all and still
/// return `Ok(())`. The symptom shows up late — the log has no directory to
/// open into, the session has nowhere to write — and reads like a
/// permissions failure rather than a directory that was never created.
///
/// It's also what gives meaning to "called at startup rather than on the
/// first write": a full disk must report itself before four hundred
/// megabytes, not after.
#[test]
fn create_lays_down_the_four_directories() {
    let root = std::env::temp_dir().join(format!(
        "mc-paths-create-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&root).ok();

    let locations = from_bases(Bases {
        data: root.join("data"),
        config: root.join("config"),
        temporary: root.join("tmp"),
    });

    for (name, path) in locations.list() {
        assert!(!path.exists(), "{name} existed before the call");
    }

    locations.create().expect("creation");

    for (name, path) in locations.list() {
        assert!(path.is_dir(), "{name} was not created: {}", path.display());
    }

    // Twice in a row: an ordinary startup finds everything already in place,
    // and `create_dir_all` must not complain about that.
    locations.create().expect("second creation");

    std::fs::remove_dir_all(&root).ok();
}
