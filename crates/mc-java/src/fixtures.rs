//! A fake Temurin on disk, and the archive that delivers it.
//!
//! Two things can only be verified by actually doing them: that a binary
//! responds to `-version` — on stderr, in quotes, in a legacy format — and
//! that an Adoptium archive unpacks into a usable directory. Both are built
//! here rather than mocked.

use std::path::{Path, PathBuf};

/// Serializes tests that write an executable and then run it.
///
/// Without this, the suite fails intermittently with `ETXTBSY`: another
/// thread that forks the process briefly inherits the write descriptor of the
/// script, and the kernel refuses to execute a file that's open for writing.
/// The lock closes this window — no write is in progress while another test
/// launches a process.
/// An atomic lock rather than a `Mutex`: half of the tests involved are
/// async, and holding a `MutexGuard` across an `await` is exactly what
/// clippy refuses — rightly, since nothing guarantees the task resumes on the
/// same thread.
static WORKSHOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Workshop;

/// Hold this while faking a runtime and running it.
pub(crate) fn workshop() -> Workshop {
    use std::sync::atomic::Ordering;
    while WORKSHOP
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    Workshop
}

impl Drop for Workshop {
    fn drop(&mut self) {
        WORKSHOP.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// A major version no system publishes.
///
/// `candidates` doesn't limit itself to the directory it's given: it also
/// probes `JAVA_HOME`, `PATH`, and `/usr/lib/jvm`. On a development machine —
/// or on a CI runner, which ships a recent JDK — a test that requires
/// "nothing was retained" would then depend on the machine's Java rather than
/// on what the test set up. Asking for a version nobody ships is the only way
/// to rule out these candidates without touching the code under test.
pub(crate) const MISSING_MAJOR: u32 = 999;

pub(crate) struct Tree {
    pub(crate) root: PathBuf,
}

impl Tree {
    pub(crate) fn new(name: &str) -> Tree {
        let root = std::env::temp_dir().join(format!(
            "mc-java-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Tree { root }
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

/// Writes an executable that responds to `-version` the way a JVM does.
///
/// The output goes to stderr and the number is in quotes: that's the format
/// `probe` parses, and it's not obvious.
#[cfg(unix)]
pub(crate) fn fake_java(path: &Path, version: &str) {
    write_executable(
        path,
        &format!(
            "#!/bin/sh\n\
             echo 'openjdk version \"{version}\"' >&2\n\
             echo 'OpenJDK Runtime Environment (build {version})' >&2\n"
        ),
    );
}

/// An executable that starts but says nothing usable: the case of a
/// half-uninstalled package, which must be ruled out without stopping on it.
#[cfg(unix)]
pub(crate) fn silent_java(path: &Path) {
    write_executable(path, "#!/bin/sh\nexit 0\n");
}

#[cfg(unix)]
fn write_executable(path: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, script).unwrap();
    // 0o700 and not 0o755: the only process that needs to run this script is
    // the one that just wrote it. Granting group and other permissions would
    // serve no purpose, and in a shared temp directory would leave an
    // executable that any account on the machine could swap out between its
    // being written and being run.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// Builds the archive Adoptium publishes: a root directory named after the
/// version, containing `bin/java`.
#[cfg(unix)]
pub(crate) fn temurin_archive(version: &str) -> Vec<u8> {
    let workspace = Tree::new(&format!("workshop-{version}"));
    let root = workspace.root.join(format!("jdk-{version}"));
    fake_java(&root.join("bin").join("java"), version);

    let mut package = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::fast(),
    ));
    package
        .append_dir_all(format!("jdk-{version}"), &root)
        .unwrap();
    package.into_inner().unwrap().finish().unwrap()
}

/// Adoptium's response for a given binary, as it gets deserialized.
pub(crate) fn adoptium_response(image: &str, link: &str, name: &str, sha256: &str) -> String {
    format!(
        r#"[{{"release_name":"jdk-21.0.5+11",
             "binary":{{"image_type":"{image}",
                        "package":{{"link":"{link}","name":"{name}","checksum":"{sha256}"}}}}}}]"#
    )
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    mc_dl::Checksum::Sha256(String::new()).of(bytes)
}
