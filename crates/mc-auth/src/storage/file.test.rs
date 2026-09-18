use super::{erase_from, load_from, path, save_to, write_protected};

fn folder(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "mc-auth-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&path).ok();
    path
}

/// The file carries a refresh token: a shared machine must not let a
/// neighbor read it.
#[cfg(unix)]
#[test]
fn the_session_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;

    let dir = folder("permissions");
    std::fs::create_dir_all(&dir).expect("test directory");
    let file = dir.join("session.json");

    write_protected(&file, b"{}").expect("write");
    let mode = std::fs::metadata(&file)
        .expect("file written")
        .permissions()
        .mode();

    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(mode & 0o777, 0o600, "mode {:o}", mode & 0o777);
}

/// The secret lives in the config directory, not the cache: it doesn't
/// rebuild itself, and clearing the data directory must not sign anyone out.
#[test]
fn the_session_lives_next_to_the_config() {
    // Expressed against the SEGMENT rather than hard-coded: what matters here
    // is that the session sits under the launcher's config, not the name
    // that directory carries — which has already changed once.
    let path = path();
    assert!(
        path.ends_with(format!("{}/session.json", mc_paths::SEGMENT)),
        "{path:?}"
    );
}

#[test]
fn what_is_written_reads_back_identically() {
    let dir = folder("round-trip");
    let file = dir.join("session.json");
    let state = serde_json::json!({
        "refresh_token": "M.R3_BAY.secret",
        "expires_at": 1_790_000_000u64
    });

    save_to(&file, &state).expect("write");
    assert_eq!(load_from(&file), Some(state));

    std::fs::remove_dir_all(&dir).ok();
}

/// The config directory may not exist on first launch: saving creates it
/// rather than failing.
#[test]
fn the_config_directory_is_created_as_needed() {
    let dir = folder("creation");
    let file = dir.join("even").join("further").join("down").join("s.json");

    save_to(&file, &serde_json::json!({})).expect("write");
    assert!(file.is_file());

    std::fs::remove_dir_all(&dir).ok();
}

/// A corrupted file means "no session": refusing to start over it would
/// prevent playing, and the only way out would be to delete the file by
/// hand.
#[test]
fn a_corrupted_session_counts_as_no_session() {
    let dir = folder("corrupted");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("session.json");
    std::fs::write(&file, b"{this is not JSON").unwrap();

    assert!(load_from(&file).is_none());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_saved_session_is_not_an_error() {
    let dir = folder("absent");
    assert!(load_from(&dir.join("session.json")).is_none());
}

/// Signing out twice in a row must stay a no-op, not fail.
#[test]
fn forgetting_an_absent_session_stays_a_no_op() {
    let dir = folder("forget");
    let file = dir.join("session.json");

    save_to(&file, &serde_json::json!({"a": 1})).unwrap();
    erase_from(&file).expect("first deletion");
    assert!(!file.exists());
    erase_from(&file).expect("second deletion");

    std::fs::remove_dir_all(&dir).ok();
}

/// A directory where a file is expected: deletion fails, and the message
/// must name the offending path.
#[test]
fn a_failed_deletion_names_the_path() {
    let dir = folder("impossible");
    std::fs::create_dir_all(dir.join("session.json")).unwrap();

    let error = erase_from(&dir.join("session.json")).expect_err("it's a directory");
    assert!(format!("{error:#}").contains("session.json"), "{error:#}");

    std::fs::remove_dir_all(&dir).ok();
}

/// All three operations read the location from the environment. Testing
/// them together checks that they designate the same file: an asymmetry
/// between writing and reading would sign the player out on every launch,
/// silently.
///
/// `super::super`'s wrappers aren't tested here: they go through the system
/// keyring first, and a test that wrote to it would touch the wallet of the
/// machine running the suite.
#[test]
fn the_file_operations_designate_the_same_location() {
    let dir = folder("wrappers");
    std::fs::create_dir_all(&dir).unwrap();

    // SAFETY: the variable is restored before the test ends, and this crate
    // spawns no subprocess.
    let previous = std::env::var_os("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &dir);
    }

    let state = serde_json::json!({"refresh_token": "M.R3_BAY.secret"});
    save_to(&path(), &state).expect("write");
    assert_eq!(path(), dir.join(mc_paths::SEGMENT).join("session.json"));
    assert_eq!(load_from(&path()), Some(state));
    erase_from(&path()).expect("deletion");
    assert!(load_from(&path()).is_none());

    unsafe {
        match previous {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
    std::fs::remove_dir_all(&dir).ok();
}
