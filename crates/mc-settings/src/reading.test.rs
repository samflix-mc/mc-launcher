use super::{load, save};
use crate::types::{Backdrop, Settings, WindowMode};

/// A clean working directory, erased on drop.
struct Workshop {
    root: std::path::PathBuf,
}

impl Workshop {
    fn new(name: &str) -> Workshop {
        let root = std::env::temp_dir().join(format!(
            "mc-settings-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Workshop { root }
    }

    fn file(&self) -> std::path::PathBuf {
        self.root.join("settings.json")
    }
}

impl Drop for Workshop {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

/// On a fresh machine, the defaults. No error, no file to create before
/// opening the window.
#[test]
fn no_file_means_the_defaults() {
    let workshop = Workshop::new("absent");
    assert_eq!(load(&workshop.file()), Settings::default());
}

#[test]
fn what_gets_saved_is_re_read() {
    let workshop = Workshop::new("round-trip");
    let mut wanted = Settings::default();
    wanted.game.render_distance = 20;
    wanted.window.mode = WindowMode::Fullscreen;
    wanted.appearance.backdrop = Backdrop::Nether;
    wanted.launcher.memory_mb = Some(8192);

    save(&workshop.file(), &wanted).expect("write");

    assert_eq!(load(&workshop.file()), wanted);
}

/// An unreadable file must not prevent opening the launcher: losing your
/// settings is annoying, not being able to play is worse.
#[test]
fn a_broken_file_gives_the_defaults() {
    let workshop = Workshop::new("broken");
    std::fs::write(workshop.file(), b"{this is not JSON").unwrap();
    assert_eq!(load(&workshop.file()), Settings::default());
}

/// A file from another schema is NOT refused: `#[serde(default)]` on each
/// section means whatever re-reads is kept, and whatever we don't know
/// falls back to its default. Losing everything would be worse.
#[test]
fn another_schema_keeps_what_re_reads() {
    let workshop = Workshop::new("schema");
    std::fs::write(
        workshop.file(),
        br#"{"schema":99,"game":{"renderDistance":18},"unknown":{"x":1}}"#,
    )
    .unwrap();

    let read = load(&workshop.file());
    assert_eq!(read.game.render_distance, 18, "what re-read was lost");
    // And the schema is brought back to today's by `validate`.
    assert_eq!(read.schema, crate::types::SCHEMA);
}

/// What gets re-read is ALWAYS validated: the file can be hand-edited, and
/// that's exactly what a player chasing frames per second will do. A render
/// distance of 200 doesn't crash the game — it makes it allocate gigabytes
/// until OutOfMemoryError, twenty minutes later.
#[test]
fn what_is_re_read_is_clamped() {
    let workshop = Workshop::new("clamped-on-read");
    std::fs::write(
        workshop.file(),
        br#"{"schema":1,"game":{"renderDistance":200,"maxFps":9999},"appearance":{"scrim":0.0}}"#,
    )
    .unwrap();

    let read = load(&workshop.file());
    assert_eq!(read.game.render_distance, super::super::bounds::RENDER.1);
    assert_eq!(read.game.max_fps, super::super::bounds::FPS.1);
    assert_eq!(read.appearance.scrim, super::super::bounds::SCRIM_FLOOR);
}

/// `save` returns what was WRITTEN and not what it received.
///
/// Without this, a value clamped into bounds would leave the window's
/// slider at a position the file doesn't carry, until the next reload —
/// and the player would think they'd set 200 when the game will receive 32.
#[test]
fn save_returns_what_was_written() {
    let workshop = Workshop::new("returns-the-valid-one");
    let mut invalid = Settings::default();
    invalid.game.render_distance = 200;

    let written = save(&workshop.file(), &invalid).expect("write");

    assert_eq!(written.game.render_distance, super::super::bounds::RENDER.1);
    assert_eq!(written, load(&workshop.file()));
}

/// The parent directory is created as needed: on the very first save, the
/// config directory may not exist yet.
#[test]
fn the_directory_is_created_as_needed() {
    let workshop = Workshop::new("parent");
    let deep = workshop.root.join("a").join("b").join("settings.json");
    save(&deep, &Settings::default()).expect("write");
    assert!(deep.is_file());
}

/// The settings file has a name, and it's under CONFIG.
///
/// Nothing checked this: `path()` could return an empty path without a
/// test flinching. The symptom would be a launcher that saves nothing and
/// re-reads the defaults on every open — without error, since `load` never
/// returns one.
#[test]
fn settings_have_a_path_under_config() {
    let where_ = super::path();

    assert!(where_.is_absolute(), "{}", where_.display());
    assert!(where_.ends_with("settings.json"), "{}", where_.display());
    assert!(
        where_.starts_with(mc_paths::current().config),
        "settings must be under config, nowhere else: {}",
        where_.display()
    );
}
