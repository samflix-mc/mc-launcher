//! Merge our keys into the game's `options.txt`.
//!
//! ## Why merge and not write
//!
//! `options.txt` belongs to Minecraft. It holds the keybinds, the volume,
//! the language, and a `version:` line the game bases its data fixers on
//! from one version to the next. Rewriting it wholesale would lose all of
//! that.
//!
//! So we only touch the keys we know, keep the others in their place and
//! order, and only append at the end what was missing.
//!
//! ## Why we don't CREATE it
//!
//! A missing `options.txt` means the game has never started on this
//! instance. Fabricating a partial one would lose the `version:` line, and
//! the game would apply its data fixers as if the file came from an unknown
//! version. The settings page tells the player instead — "these settings
//! will apply after your first session" — rather than silently failing.
//!
//! ## Why it's a pure function
//!
//! It takes text and returns text. Everything else — reading, writing,
//! refusing mid-session — belongs to the caller. That's what lets the cases
//! that matter be tested: a key already there, a missing key, a file with
//! lines we don't know, an empty file.

use crate::types::{Game, Window};

/// The keys we drive, and nothing else.
///
/// `graphicsMode` is deliberately absent from it: shaders replace it, and
/// forcing it from the launcher would undo what Iris set.
fn our_keys(game: &Game, window: &Window) -> Vec<(&'static str, String)> {
    vec![
        ("renderDistance", game.render_distance.to_string()),
        ("simulationDistance", game.simulation_distance.to_string()),
        (
            "maxFps",
            // Above 260, Minecraft doesn't expect a number but the word
            // "max". Writing 261 would be read back as invalid and the game
            // would fall back to its default, without saying anything.
            if game.max_fps >= 260 {
                "260".to_string()
            } else {
                game.max_fps.to_string()
            },
        ),
        ("guiScale", game.gui_scale.to_string()),
        ("enableVsync", game.vsync.to_string()),
        // Derived from the mode: see `Window::fullscreen`.
        ("fullscreen", window.fullscreen().to_string()),
    ]
}

/// Returns `options.txt`'s content with our keys up to date.
///
/// Unknown keys are kept in their place and order; ours are replaced in
/// place where they exist, and appended at the end otherwise.
pub fn merge(existing: &str, game: &Game, window: &Window) -> String {
    let wanted = our_keys(game, window);
    let mut placed = vec![false; wanted.len()];

    let mut lines: Vec<String> = Vec::new();
    for line in existing.lines() {
        // `options.txt` is "key:value", one pair per line. A line without a
        // colon isn't a key: we keep it as-is rather than guessing.
        let key = line.split_once(':').map(|(key, _)| key);
        match key.and_then(|key| wanted.iter().position(|(name, _)| *name == key)) {
            Some(rank) => {
                lines.push(format!("{}:{}", wanted[rank].0, wanted[rank].1));
                placed[rank] = true;
            }
            None => lines.push(line.to_string()),
        }
    }

    for (rank, (name, value)) in wanted.iter().enumerate() {
        if !placed[rank] {
            lines.push(format!("{name}:{value}"));
        }
    }

    let mut text = lines.join("\n");
    // Minecraft always writes a trailing newline. Not restoring it would
    // make the file change shape on every round trip between it and us.
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

#[cfg(test)]
#[path = "options_txt.test.rs"]
mod tests;
