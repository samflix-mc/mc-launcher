//! Under what name the launcher presents itself.
//!
//! **"Helm", and that's the name of the LAUNCHER — not the network's.** The
//! distinction is the one the design system draws: the launcher is
//! server-agnostic, the samflix-mc network is its first tenant, and the
//! server's name shows up next to the wordmark, not in its place.
//!
//! The name stays configurable: writing it into the HTML template and into
//! `tauri.conf.json` would force going through both to change it, and risk
//! forgetting one.
//!
//! So it's read **at compile time**, from `MC_LAUNCHER_NOM`:
//!
//! ```sh
//! MC_LAUNCHER_NOM="My Network" cargo tauri build
//! ```
//!
//! ## Why at compile time and not at launch
//!
//! An environment variable read at startup would make the name change
//! depending on the shell it's clicked from — which is to say, never for a
//! player, who double-clicks an icon. The name belongs to the binary being
//! distributed, just like its icon: it's fixed when it's built.
//!
//! `build.rs` declares the variable to Cargo, without which changing it
//! wouldn't trigger a recompile and the binary would keep the old name.

use serde::Serialize;

/// What the launcher displays about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Brand {
    /// The full name, in the window title and in the header.
    pub name: String,
    /// Two letters for the round seal.
    pub seal: String,
}

/// The chosen name, fixed at compile time.
pub fn name() -> &'static str {
    // `option_env!` and not `env!`: a build without the variable must
    // succeed, and fall back to today's name.
    match option_env!("MC_LAUNCHER_NOM") {
        Some(name) if !name.is_empty() => name,
        _ => "Helm",
    }
}

/// The two letters of the seal.
///
/// Derived from the name rather than set separately: a changed name without
/// its seal would give a circle that contradicts the header right next to
/// it.
pub fn seal(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();

    let letters: String = match words.as_slice() {
        // A single word: its first two letters. "samflix" → "SA".
        [only] => only.chars().take(2).collect(),
        // Several: the initials of the first two. "My Network" → "MN".
        [first, second, ..] => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
        [] => String::new(),
    };

    if letters.is_empty() {
        // A name without a single letter or digit remains possible; an
        // empty seal would make a silent circle.
        return "??".to_string();
    }
    letters.to_uppercase()
}

impl Brand {
    pub fn current() -> Self {
        let name = name();
        Self {
            name: name.to_string(),
            seal: seal(name),
        }
    }
}

#[cfg(test)]
#[path = "brand.test.rs"]
mod tests;
