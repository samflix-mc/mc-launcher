//! A complete on-disk installation, as seen from the binary.
//!
//! Commands read a pack, a lock, and an instance: building all three at once
//! is what every test suite would otherwise re-ask for separately.

use std::path::PathBuf;

use mc_pack::lockfile::{LockedLoader, LockedMod, Lockfile};

pub(super) struct Workshop {
    pub(super) root: PathBuf,
}

impl Workshop {
    pub(super) fn new(name: &str) -> Workshop {
        let root = std::env::temp_dir().join(format!(
            "mc-pack-bin-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Workshop { root }
    }

    pub(super) fn options(&self) -> mc_pack::Options {
        mc_pack::Options {
            layout: mc_instance::Layout::new(self.root.join("data")),
            instance_name: Some("samflix".into()),
            ..Default::default()
        }
    }

    /// Writes the manifest, its lock, and what `mc_instance::verify` requires.
    pub(super) fn installed_pack(&self, mods: Vec<LockedMod>) -> mc_pack::source::Source {
        let manifest = self.root.join("samflix.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        lockfile(mods.clone())
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

        mc_pack::source::Source::parse(manifest.to_str().unwrap(), &options.layout)
    }
}

impl Drop for Workshop {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

pub(super) const JAR: &[u8] = b"the jar";

pub(super) const MANIFEST: &str = r#"{"schema":1,"name":"samflix","minecraft":"1.21.1",
  "loader":{"type":"neoforge","version":"21.1.250"},
  "servers":{"production":{"host":"mc.ggy.info"},
             "development":{"host":"78.46.100.5","port":25566}}}"#;

pub(super) fn lockfile(mods: Vec<LockedMod>) -> Lockfile {
    Lockfile {
        schema: 1,
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

pub(super) fn entry(slug: &str, side: &str) -> LockedMod {
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
        sha1: Some(mc_dl::Checksum::Sha1(String::new()).of(JAR)),
        sha512: None,
        size: JAR.len() as u64,
        side: side.to_string(),
        reason: "requested".into(),
        provides: vec![slug.to_string()],
    }
}

/// Serializes the tests that set `SAMFLIX_ENV`.
///
/// The environment is read by everything that picks a server or logs a
/// command: two tests that change it at the same time would contradict each
/// other. An atomic lock, not a `Mutex`, so it also holds across async tests.
static ENVIRONMENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) struct EnvironmentGuard;

/// Sets the deployment environment, and clears it when dropped.
pub(super) fn environment(value: &str) -> EnvironmentGuard {
    use std::sync::atomic::Ordering;
    while ENVIRONMENT
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    let guard = EnvironmentGuard;
    guard.set(value);
    guard
}

impl EnvironmentGuard {
    pub(super) fn set(&self, value: &str) {
        // SAFETY: the lock guarantees no other test in this binary reads or
        // writes SAMFLIX_ENV while the guard is alive.
        unsafe {
            std::env::set_var("SAMFLIX_ENV", value);
        }
    }
}

impl Drop for EnvironmentGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("SAMFLIX_ENV");
        }
        ENVIRONMENT.store(false, std::sync::atomic::Ordering::Release);
    }
}
