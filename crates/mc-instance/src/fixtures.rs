//! A fake installation on disk, to exercise whatever reads it.
//!
//! Assembling the command line, verification, and the classpath don't just
//! work with in-memory structures: they read `version.json` files, require
//! that the jars exist, and refuse to continue if one is missing. That's
//! precisely the refusal being tested here, so real files are needed.
//!
//! The tree lives in the temp directory, carries the process number and the
//! thread number, and disappears on drop — two tests running in parallel
//! don't step on each other.

use std::path::PathBuf;

pub(crate) struct Tree {
    pub(crate) root: PathBuf,
}

impl Tree {
    pub(crate) fn new(name: &str) -> Tree {
        let root = std::env::temp_dir().join(format!(
            "mc-instance-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Tree { root }
    }

    pub(crate) fn shared(&self) -> PathBuf {
        self.root.join("shared")
    }

    pub(crate) fn game_dir(&self) -> PathBuf {
        self.root.join("instance").join("minecraft")
    }

    /// Writes `versions/<id>/<id>.json`.
    pub(crate) fn version(&self, id: &str, json: &str) -> &Self {
        self.write(
            &self
                .shared()
                .join("versions")
                .join(id)
                .join(format!("{id}.json")),
            json.as_bytes(),
        );
        self
    }

    /// Writes `versions/<id>/<id>.jar`, the client the classpath requires.
    pub(crate) fn client(&self, id: &str) -> &Self {
        self.write(
            &self
                .shared()
                .join("versions")
                .join(id)
                .join(format!("{id}.jar")),
            b"jar",
        );
        self
    }

    /// Writes a library, path relative to `libraries/`.
    pub(crate) fn library(&self, relative: &str) -> &Self {
        self.write(&self.shared().join("libraries").join(relative), b"jar");
        self
    }

    /// Stores an asset object under its digest, and returns it.
    pub(crate) fn asset(&self, content: &[u8]) -> String {
        let draft = self.root.join("draft");
        std::fs::write(&draft, content).unwrap();
        let digest = mc_dl::sha1_of_file(&draft).unwrap();
        let destination = self
            .shared()
            .join("assets")
            .join("objects")
            .join(&digest[..2])
            .join(&digest);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::rename(&draft, &destination).unwrap();
        digest
    }

    /// Writes `assets/indexes/<id>.json` naming the given digests.
    pub(crate) fn index_assets(&self, id: &str, digests: &[String]) -> &Self {
        let objects: Vec<String> = digests
            .iter()
            .enumerate()
            .map(|(rank, digest)| format!(r#""object{rank}":{{"hash":"{digest}","size":3}}"#))
            .collect();
        self.write(
            &self
                .shared()
                .join("assets")
                .join("indexes")
                .join(format!("{id}.json")),
            format!(r#"{{"objects":{{{}}}}}"#, objects.join(",")).as_bytes(),
        );
        self
    }

    fn write(&self, path: &std::path::Path, content: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

/// A vanilla descriptor trimmed down to what matters, modeled on the one
/// Mojang publishes: a library with no rule, one reserved for another
/// system, and arguments where some are conditional.
pub(crate) const VANILLA: &str = r#"{
  "id": "1.21.1",
  "mainClass": "net.minecraft.client.main.Main",
  "assetIndex": { "id": "17" },
  "libraries": [
    { "name": "com.google.guava:guava:32.1.2-jre",
      "downloads": { "artifact": { "path": "com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar",
                                   "sha1": "aa", "size": 1, "url": "https://example.invalid/g.jar" } } },
    { "name": "org.lwjgl:lwjgl:3.3.3:natives-macos",
      "rules": [ { "action": "allow", "os": { "name": "osx" } } ] }
  ],
  "arguments": {
    "jvm": [ "-Djava.library.path=${natives_directory}", "-cp", "${classpath}" ],
    "game": [
      "--username", "${auth_player_name}",
      "--uuid", "${auth_uuid}",
      { "rules": [ { "action": "allow", "features": { "is_quick_play_multiplayer": true } } ],
        "value": [ "--quickPlayMultiplayer", "${quickPlayMultiplayer}" ] },
      { "rules": [ { "action": "allow", "features": { "has_custom_resolution": true } } ],
        "value": [ "--width", "${resolution_width}", "--height", "${resolution_height}" ] }
    ]
  }
}"#;

/// Serializes the tests that write an executable, or that launch one.
///
/// Without this, the suite fails intermittently on `ETXTBSY` — "Text file
/// busy". One test writes a script and runs it; another, at the same
/// instant, forks the process to launch `/bin/sh`. The child briefly
/// inherits the still-open write descriptor, and the kernel refuses to
/// execute a file someone else holds open for writing. Nothing in the code
/// under test is at fault, and the failure only shows up under load: it
/// took seven machines running in parallel to trigger it.
///
/// The lock closes the window on both sides — no write while another test
/// launches a process, and vice versa. It's the same guard, and the same
/// `ETXTBSY`, that mc-java holds in its own fixtures module.
///
/// An atomic lock rather than a `Mutex`: these tests are async, and holding
/// a `MutexGuard` across an `await` is exactly what clippy refuses — rightly
/// so, since nothing guarantees the task resumes on the same thread.
static WORKSHOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Workshop;

/// To hold while building an executable, or while launching one.
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

/// A loader's delta: it inherits from the base and replaces one library.
pub(crate) const NEOFORGE: &str = r#"{
  "id": "neoforge-21.1.250",
  "inheritsFrom": "1.21.1",
  "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
  "libraries": [
    { "name": "com.google.guava:guava:33.0.0-jre" },
    { "name": "net.neoforged.fancymodloader:loader:4.0.24" }
  ],
  "arguments": {
    "jvm": [ "-DlibraryDirectory=${library_directory}" ],
    "game": [ "--launchTarget", "neoforgeclient" ]
  }
}"#;
