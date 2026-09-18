//! A pack on disk, and something to serve it with.
//!
//! Everything this crate does starts from a manifest and a lockfile: reading
//! them, writing them, checking them against what's installed. Building them
//! by hand in every suite would drown what each one actually verifies — a
//! lockfile has eight fields and each mod has fourteen.

use std::path::PathBuf;

use crate::lockfile::{LockedLoader, LockedMissing, LockedMod, Lockfile};

/// A clean working directory, erased on drop.
pub(crate) struct Workshop {
    pub(crate) root: PathBuf,
}

impl Workshop {
    pub(crate) fn new(name: &str) -> Workshop {
        let root = std::env::temp_dir().join(format!(
            "mc-pack-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Workshop { root }
    }

    pub(crate) fn write(&self, relative: &str, content: &[u8]) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    pub(crate) fn options(&self) -> crate::Options {
        crate::Options {
            layout: mc_instance::Layout::new(self.root.join("data")),
            instance_name: Some("samflix".into()),
            ..Default::default()
        }
    }

    /// Writes the manifest, its lockfile, and what `mc_instance::verify`
    /// requires.
    ///
    /// The launch never goes fetch the published pack: it opens what's laid
    /// down on the machine. A suite that checks it must therefore lay down a
    /// full pack, not just a manifest.
    pub(crate) fn installed_pack(&self, mods: Vec<LockedMod>) -> crate::source::Source {
        let manifest_path = self.root.join("samflix.json");
        std::fs::write(&manifest_path, MANIFEST).unwrap();
        lock(mods.clone())
            .save(&self.root.join("samflix.lock.json"))
            .unwrap();

        let options = self.options();
        let shared = options.layout.shared();
        for (path, content) in [
            ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
            ("versions/1.21.1/1.21.1.jar", "jar"),
            (
                "versions/neoforge-21.1.250/neoforge-21.1.250.json",
                r#"{"libraries":[]}"#,
            ),
        ] {
            let target = shared.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, content).unwrap();
        }

        let instance = options.layout.instance("samflix");
        std::fs::create_dir_all(instance.mods_dir()).unwrap();
        std::fs::create_dir_all(instance.dir.join("server").join("mods")).unwrap();
        for entry in &mods {
            for folder in [
                instance.mods_dir(),
                instance.dir.join("server").join("mods"),
            ] {
                std::fs::write(folder.join(&entry.file_name), JAR).unwrap();
            }
        }

        crate::source::Source::parse(manifest_path.to_str().unwrap(), &options.layout)
    }
}

/// What we write in place of a real jar.
pub(crate) const JAR: &[u8] = b"the jar";

impl Drop for Workshop {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

/// The minimal manifest the check accepts.
pub(crate) const MANIFEST: &str = r#"{"schema":1,"name":"samflix","minecraft":"1.21.1",
  "loader":{"type":"neoforge","version":"21.1.250"},
  "servers":{"production":{"host":"mc.ggy.info"},
             "development":{"host":"78.46.100.5","port":25566}}}"#;

/// A lockfile, with the mods it's given.
pub(crate) fn lock(mods: Vec<LockedMod>) -> Lockfile {
    Lockfile {
        schema: crate::manifest::SCHEMA,
        name: "samflix".into(),
        version: None,
        generated: "2026-09-17T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        generation: 0,
        servers: Default::default(),
        mods,
        unresolved: Vec::new(),
    }
}

/// One lockfile line: a mod, its side, its digest.
pub(crate) fn entry(slug: &str, side: &str, content: Option<&[u8]>) -> LockedMod {
    LockedMod {
        slug: slug.to_string(),
        name: slug.to_string(),
        source: mc_mods::Origin::Modrinth,
        project: format!("{slug}-id"),
        file: format!("{slug}-1.0"),
        version: "1.0".into(),
        channel: mc_mods::Channel::Release,
        file_name: format!("{slug}.jar"),
        url: format!("https://exemple.invalid/{slug}.jar"),
        sha1: content.map(|c| mc_dl::Checksum::Sha1(String::new()).of(c)),
        sha512: None,
        size: content.map(|c| c.len() as u64).unwrap_or(0),
        side: side.to_string(),
        reason: "requested".into(),
        provides: vec![slug.to_string()],
    }
}

pub(crate) fn missing(mod_id: &str, required_by: &str) -> LockedMissing {
    LockedMissing {
        mod_id: mod_id.to_string(),
        required_by: required_by.to_string(),
        side: "both".into(),
    }
}

/// Serializes the tests that set `SAMFLIX_ENV`.
///
/// The environment is read by everything that picks a server or logs a
/// command: two tests that change it at the same time contradict each other.
/// An atomic lock, not a `Mutex`, so that it also spans async tests.
static ENVIRONMENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Environment;

/// Sets the deployment environment, and clears it on drop.
pub(crate) fn environment(value: &str) -> Environment {
    use std::sync::atomic::Ordering;
    while ENVIRONMENT
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    let guard = Environment;
    guard.set(value);
    guard
}

impl Environment {
    pub(crate) fn set(&self, value: &str) {
        // SAFETY: the lock guarantees that no other test in this binary reads
        // or writes SAMFLIX_ENV while the guard is alive.
        unsafe {
            std::env::set_var("SAMFLIX_ENV", value);
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("SAMFLIX_ENV");
        }
        ENVIRONMENT.store(false, std::sync::atomic::Ordering::Release);
    }
}
