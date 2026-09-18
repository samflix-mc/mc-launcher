use super::merge;
use crate::types::{Game, Window, WindowMode};

fn key(text: &str, name: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{name}:")))
        .map(str::to_string)
}

/// THE property of this module: what isn't ours stays intact, in its place
/// and in its order.
///
/// `version:` in particular — the game bases its data fixers on it, and
/// losing it would make it treat the file as coming from an unknown
/// version.
#[test]
fn what_is_not_ours_stays_intact() {
    let before =
        "version:3955\nlang:fr_fr\nkey_key.attack:key.mouse.left\nsoundCategory_master:0.7\n";

    let after = merge(before, &Game::default(), &Window::default());

    assert_eq!(key(&after, "version").as_deref(), Some("3955"));
    assert_eq!(key(&after, "lang").as_deref(), Some("fr_fr"));
    assert_eq!(
        key(&after, "key_key.attack").as_deref(),
        Some("key.mouse.left")
    );
    assert_eq!(key(&after, "soundCategory_master").as_deref(), Some("0.7"));
    // And in their order: the four original lines still open the file.
    let lines: Vec<&str> = after.lines().take(4).collect();
    assert_eq!(lines[0], "version:3955");
    assert_eq!(lines[1], "lang:fr_fr");
}

/// A key that already exists is replaced IN PLACE, not appended at the end.
/// A file that gained a duplicate on every save would end up with hundreds
/// of lines, and Minecraft would only read the first.
#[test]
fn a_key_already_there_is_replaced_in_place_and_only_once() {
    let before = "renderDistance:8\nlang:fr_fr\n";
    let game = Game {
        render_distance: 16,
        ..Game::default()
    };

    let after = merge(before, &game, &Window::default());

    assert_eq!(
        after
            .lines()
            .filter(|l| l.starts_with("renderDistance:"))
            .count(),
        1,
        "{after}"
    );
    assert_eq!(key(&after, "renderDistance").as_deref(), Some("16"));
    // In place: it still opens the file.
    assert!(after.starts_with("renderDistance:16\n"), "{after}");
}

/// A missing key gets appended at the end.
#[test]
fn a_missing_key_is_appended() {
    let after = merge("lang:fr_fr\n", &Game::default(), &Window::default());
    assert_eq!(
        key(&after, "renderDistance").as_deref(),
        Some(&*Game::default().render_distance.to_string())
    );
}

/// An empty file gives our keys, and nothing else.
#[test]
fn an_empty_file_gives_our_keys() {
    let after = merge("", &Game::default(), &Window::default());
    assert!(key(&after, "renderDistance").is_some());
    assert!(key(&after, "fullscreen").is_some());
    assert!(after.ends_with('\n'));
}

/// A line without a colon isn't a key: we keep it rather than guessing.
/// Files from older versions carry some.
#[test]
fn a_line_without_a_colon_is_kept() {
    let before = "a weird line\nlang:fr_fr\n";
    let after = merge(before, &Game::default(), &Window::default());
    assert!(after.contains("a weird line"), "{after}");
}

/// `fullscreen` follows the window MODE, not a separate field. That's what
/// keeps the two from diverging when the player hits F11 mid-session.
#[test]
fn the_written_fullscreen_follows_the_mode() {
    for (mode, expected) in [
        (WindowMode::Windowed, "false"),
        (WindowMode::Maximized, "false"),
        (WindowMode::Fullscreen, "true"),
    ] {
        let window = Window {
            mode,
            ..Window::default()
        };
        let after = merge("", &Game::default(), &window);
        assert_eq!(
            key(&after, "fullscreen").as_deref(),
            Some(expected),
            "{mode:?}"
        );
    }
}

/// Above 260, Minecraft doesn't expect a number but the word "max": writing
/// 261 would be read back as invalid and the game would fall back to its
/// default, without saying anything. So we cap at 260.
#[test]
fn the_frame_cap_never_exceeds_two_hundred_sixty() {
    let game = Game {
        max_fps: 260,
        ..Game::default()
    };
    let after = merge("", &game, &Window::default());
    assert_eq!(key(&after, "maxFps").as_deref(), Some("260"));
}

/// `graphicsMode` is NOT written: shaders drive it, and forcing it from the
/// launcher would undo what Iris set.
#[test]
fn the_graphics_mode_is_never_forced() {
    let before = "graphicsMode:0\n";
    let after = merge(before, &Game::default(), &Window::default());
    assert_eq!(key(&after, "graphicsMode").as_deref(), Some("0"));
}

/// Two merges in a row give the same text: without this property, the file
/// would change shape every time the settings page opens, and a player who
/// versions their instance would see a diff without having touched
/// anything.
#[test]
fn merging_twice_changes_nothing_further() {
    let before = "version:3955\nlang:fr_fr\nrenderDistance:8\n";
    let once = merge(before, &Game::default(), &Window::default());
    let twice = merge(&once, &Game::default(), &Window::default());
    assert_eq!(once, twice);
}
