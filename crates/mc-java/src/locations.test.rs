use super::{candidates, default_runtime_dir, java_exe, managed_home};
use std::path::Path;

/// The runtime installed is dedicated to the launcher: it lives in its data
/// directory, isn't added to `PATH`, and doesn't touch the system's Java.
#[test]
fn the_launcher_runtime_lives_under_its_own_data() {
    assert!(default_runtime_dir().ends_with("runtime"));
    assert!(default_runtime_dir().starts_with(mc_paths::current().data));
}

#[test]
fn each_major_version_has_its_own_directory() {
    let dir = Path::new("/data/runtime");
    assert_eq!(managed_home(dir, 21), Path::new("/data/runtime/temurin-21"));
    assert_ne!(managed_home(dir, 21), managed_home(dir, 17));
}

#[cfg(unix)]
#[test]
fn the_executable_lives_under_bin() {
    assert_eq!(
        java_exe(Path::new("/data/runtime/temurin-21")),
        Path::new("/data/runtime/temurin-21/bin/java")
    );
}

/// The managed runtime always opens the list: if it's there, the launcher
/// installed and verified it, no need to probe the system.
///
/// The environment isn't touched here: this crate launches subprocesses, and
/// setting a variable while another thread forks the process is exactly the
/// race a test suite doesn't want.
#[test]
fn the_managed_runtime_always_opens_the_list() {
    let list = candidates(Path::new("/data/runtime"), 21);
    assert_eq!(list[0], java_exe(Path::new("/data/runtime/temurin-21")));
}

/// Every candidate names a `java` executable, never a directory: detection
/// passes them to `probe`, which runs them.
#[test]
fn each_candidate_names_a_java_executable() {
    for candidate in candidates(Path::new("/data/runtime"), 21) {
        let name = candidate.file_name().unwrap().to_string_lossy().to_string();
        assert!(
            name == "java" || name == "java.exe",
            "{}",
            candidate.display()
        );
    }
}
