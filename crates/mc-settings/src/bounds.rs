//! What a setting is allowed to be worth.
//!
//! ## Why validate in Rust and not in the form
//!
//! The form already validates — an `<input type="range">` doesn't render an
//! out-of-bounds value. But the file itself can be hand-edited, and that's
//! exactly what a player chasing frames per second will do. A render distance
//! of 200 chunks doesn't crash the game: it makes it allocate gigabytes until
//! `OutOfMemoryError`, twenty minutes later, with nothing linking cause to
//! effect.
//!
//! So we clamp into bounds instead of refusing: a file out of bounds must not
//! prevent playing.

use crate::types::{Appearance, Game, Launcher, Settings, Window};

/// Render distance: 2 chunks minimum — below that, the player doesn't see
/// the ground under their feet on load.
pub const RENDER: (u8, u8) = (2, 32);

/// Simulation distance: 5 minimum, that's the game's own floor. Below that,
/// entities stop updating around the player.
pub const SIMULATION: (u8, u8) = (5, 32);

/// Frames per second: above 260, Minecraft writes `max` and stops limiting
/// entirely. Below 10, the game becomes unplayable with no clue why.
pub const FPS: (u16, u16) = (10, 260);

/// UI scale: 0 means "automatic", otherwise 1 to 4. A value above 4 makes
/// the inventory bigger than the screen.
pub const SCALE: (u8, u8) = (0, 4);

/// JVM memory, in megabytes. 1 GB is the floor below which a modpack won't
/// start.
///
/// The ceiling used to be 32 GB, on the idea that beyond that you'd allocate
/// more than the machine has. The idea was right, the number wasn't: a 64 GB
/// machine — which is what Sam has — saw its slider stop at 32, with nothing
/// saying why. A fixed ceiling can't hold that reasoning; only the machine's
/// total could, and the launcher doesn't know it yet.
///
/// 64 GB is therefore a SAFETY bound and not a recommendation: it stops an
/// absurd value from reaching the JVM, without claiming to know what the
/// machine carries. The day the total gets read — see task R6 — that's what
/// will bound it, and this constant goes back to being a plain safeguard.
pub const MEMORY: (u32, u32) = (1024, 65_536);

/// Game window size. The floor is the point below which Minecraft's UI
/// overlaps itself.
pub const WIDTH: (u32, u32) = (640, 7680);
pub const HEIGHT: (u32, u32) = (480, 4320);

/// The scrim's floor.
///
/// **It's zero, and that's a design decision — not a concession.**
///
/// The two previous versions were 0.35, then 0.44. Both answered the same
/// question: "at what opacity does the text hold 4.5:1 against the lightest
/// conceivable image?" The answer was right; it was the QUESTION that no
/// longer was.
///
/// It assumed contrast gets tuned by darkening the image. The design system
/// answers differently, and better: "on a light image, raise the glass base
/// to 60% of the background at the window root instead of darkening the
/// text." In other words, what gets thickened is the PANEL, not the image.
///
/// The difference isn't theoretical. A scrim at 0.44 applies everywhere,
/// including where there's no text at all — that is, the middle of the
/// screen, which the design system leaves empty on purpose so the image
/// shows. The launcher was therefore displaying a black rectangle exactly
/// where its whole art direction exists to be seen.
///
/// The contrast guarantee hasn't disappeared, it moved to a different layer.
/// It's held in three places, all in `web/src/`:
///
/// 1. `.hm-stage__art::after`, the design system's gradient — the background
///    at 52% at the top, 36% in the middle, 80% at the bottom. It's what
///    keeps the bottom bar readable no matter the image, and it isn't
///    adjustable.
/// 2. `--glass-base`, which THICKENS as the scrim thins: from 46% of the
///    background to 72%. At zero scrim and against a white image, `ink-3` —
///    the lightest hue the design system allows — still holds 4.5:1 on a
///    panel. That's the countermeasure the design system prescribes, applied
///    automatically.
/// 3. The drop shadows on text that has no panel under it: tile titles and
///    the play button's hint.
///
/// The scrim thus goes back to being what an appearance setting should be: a
/// preference, one that can't make the interface unreadable because it's no
/// longer the thing making it readable.
///
/// Reopen this if `--glass-base`'s compensation gets removed — and if so,
/// it's point 2's measurement that needs redoing, not the earlier one.
pub const SCRIM_FLOOR: f32 = 0.0;

fn clamp<T: PartialOrd>(value: T, bounds: (T, T)) -> T {
    if value < bounds.0 {
        bounds.0
    } else if value > bounds.1 {
        bounds.1
    } else {
        value
    }
}

impl Settings {
    /// Brings everything back into bounds. Never refuses.
    pub fn validate(&mut self) {
        self.schema = crate::types::SCHEMA;
        self.game.validate();
        self.window.validate();
        self.launcher.validate();
        self.appearance.validate();
    }
}

impl Game {
    pub fn validate(&mut self) {
        self.render_distance = clamp(self.render_distance, RENDER);
        self.simulation_distance = clamp(self.simulation_distance, SIMULATION);
        self.max_fps = clamp(self.max_fps, FPS);
        self.gui_scale = clamp(self.gui_scale, SCALE);
    }
}

impl Window {
    pub fn validate(&mut self) {
        self.width = clamp(self.width, WIDTH);
        self.height = clamp(self.height, HEIGHT);
    }
}

impl Launcher {
    pub fn validate(&mut self) {
        // `None` is left as-is: it's a choice — "let the JVM decide" — and
        // not an out-of-bounds value.
        self.memory_mb = self.memory_mb.map(|mb| clamp(mb, MEMORY));
    }
}

impl Appearance {
    pub fn validate(&mut self) {
        // A NaN would slip through an ordinary comparison: `NaN < x` and
        // `NaN > x` are both false, and the value would come out unchanged.
        // It would then render a transparent scrim, hence an unreadable
        // interface — exactly what the floor exists to prevent.
        if !self.scrim.is_finite() {
            self.scrim = Appearance::default().scrim;
            return;
        }
        self.scrim = clamp(self.scrim, (SCRIM_FLOOR, 1.0));
    }
}

#[cfg(test)]
#[path = "bounds.test.rs"]
mod tests;
