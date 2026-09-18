use super::{FPS, MEMORY, RENDER, SCALE, SCRIM_FLOOR, SIMULATION, WIDTH};
use crate::types::{Appearance, Game, Launcher, Settings, Window, WindowMode};

/// One test per bound, AND one at the bound ± 1.
///
/// That's what the repo's test conventions require of comparisons, and
/// without it a mutant that replaces `<` with `<=` survives forever: at the
/// exact bound, both versions render the same thing.
#[test]
fn the_render_distance_is_clamped_at_both_ends() {
    let cases = [
        (0u8, RENDER.0),              // way below
        (RENDER.0 - 1, RENDER.0),     // just below
        (RENDER.0, RENDER.0),         // at the bound: unchanged
        (RENDER.0 + 1, RENDER.0 + 1), // just above: unchanged
        (RENDER.1 - 1, RENDER.1 - 1),
        (RENDER.1, RENDER.1),
        (255, RENDER.1), // a player trying to see far
    ];
    for (given, expected) in cases {
        let mut game = Game {
            render_distance: given,
            ..Game::default()
        };
        game.validate();
        assert_eq!(game.render_distance, expected, "render {given}");
    }
}

#[test]
fn the_simulation_distance_is_clamped_at_both_ends() {
    for (given, expected) in [
        (0u8, SIMULATION.0),
        (SIMULATION.0 - 1, SIMULATION.0),
        (SIMULATION.0, SIMULATION.0),
        (SIMULATION.0 + 1, SIMULATION.0 + 1),
        (SIMULATION.1, SIMULATION.1),
        (200, SIMULATION.1),
    ] {
        let mut game = Game {
            simulation_distance: given,
            ..Game::default()
        };
        game.validate();
        assert_eq!(game.simulation_distance, expected, "simulation {given}");
    }
}

/// Above 260, Minecraft writes `max` and stops limiting entirely: a player
/// who sets 9999 would think they'd asked for a high limit and would have
/// none at all.
#[test]
fn the_frame_cap_is_clamped_at_both_ends() {
    for (given, expected) in [
        (0u16, FPS.0),
        (FPS.0 - 1, FPS.0),
        (FPS.0, FPS.0),
        (FPS.0 + 1, FPS.0 + 1),
        (FPS.1, FPS.1),
        (FPS.1 + 1, FPS.1),
        (9999, FPS.1),
    ] {
        let mut game = Game {
            max_fps: given,
            ..Game::default()
        };
        game.validate();
        assert_eq!(game.max_fps, expected, "fps {given}");
    }
}

/// Zero is a VALUE and not an absence: it's "automatic scale". Clamping it
/// to 1 would take away the game's own default setting from the player.
#[test]
fn scale_zero_is_a_value_and_not_an_absence() {
    let mut game = Game {
        gui_scale: 0,
        ..Game::default()
    };
    game.validate();
    assert_eq!(game.gui_scale, 0);

    let mut too_much = Game {
        gui_scale: 9,
        ..Game::default()
    };
    too_much.validate();
    assert_eq!(too_much.gui_scale, SCALE.1);
}

/// `None` isn't an out-of-bounds value: it's the choice "let the JVM
/// decide". Replacing it with the floor would take an option away from the
/// player.
#[test]
fn absent_memory_stays_absent() {
    let mut launcher = Launcher {
        memory_mb: None,
        ..Launcher::default()
    };
    launcher.validate();
    assert_eq!(launcher.memory_mb, None);
}

#[test]
fn memory_is_clamped_at_both_ends() {
    for (given, expected) in [
        (0u32, MEMORY.0),
        (MEMORY.0 - 1, MEMORY.0),
        (MEMORY.0, MEMORY.0),
        (MEMORY.0 + 1, MEMORY.0 + 1),
        (MEMORY.1, MEMORY.1),
        (MEMORY.1 + 1, MEMORY.1),
    ] {
        let mut launcher = Launcher {
            memory_mb: Some(given),
            ..Launcher::default()
        };
        launcher.validate();
        assert_eq!(launcher.memory_mb, Some(expected), "memory {given}");
    }
}

#[test]
fn the_window_size_is_clamped() {
    let mut window = Window {
        width: 1,
        height: 1,
        ..Window::default()
    };
    window.validate();
    assert_eq!(window.width, WIDTH.0);

    let mut huge = Window {
        width: u32::MAX,
        height: u32::MAX,
        ..Window::default()
    };
    huge.validate();
    assert_eq!(huge.width, WIDTH.1);
}

// --- The scrim, and it's the bound that counts ------------------------------

/// A value BELOW the floor comes back up to the floor.
///
/// The floor has been zero since contrast started being held by the glass
/// and not the scrim — see `SCRIM_FLOOR`. This test no longer guards an
/// accessibility guarantee; it guards the fact that a negative value, which
/// a hand-edited file can carry, doesn't go down into the CSS. A negative
/// opacity would be silently ignored there, and the setting would stop
/// having an effect without anything saying so.
#[test]
fn the_scrim_does_not_go_below_the_floor() {
    for given in [-1.0, -0.01, SCRIM_FLOOR - 0.01] {
        let mut appearance = Appearance {
            scrim: given,
            ..Appearance::default()
        };
        appearance.validate();
        assert_eq!(appearance.scrim, SCRIM_FLOOR, "scrim {given}");
    }
}

/// At the bound and above, nothing moves.
///
/// Zero included: it's a value the player can ask for, and the launcher
/// must not take it back — the image is then seen in full, which is exactly
/// what the slider promises at that end.
#[test]
fn the_scrim_at_the_floor_and_above_does_not_move() {
    for given in [SCRIM_FLOOR, SCRIM_FLOOR + 0.01, 0.3, 0.9, 1.0] {
        let mut appearance = Appearance {
            scrim: given,
            ..Appearance::default()
        };
        appearance.validate();
        assert_eq!(appearance.scrim, given, "scrim {given}");
    }
}

#[test]
fn the_scrim_does_not_exceed_one() {
    let mut appearance = Appearance {
        scrim: 3.0,
        ..Appearance::default()
    };
    appearance.validate();
    assert_eq!(appearance.scrim, 1.0);
}

/// A NaN slips through an ordinary comparison unseen: `NaN < x` and
/// `NaN > x` are both false. Without the explicit check, it would come out
/// unchanged and render a transparent scrim — exactly what the floor exists
/// to prevent. A hand-edited JSON file can produce one.
#[test]
fn a_scrim_that_is_not_a_number_falls_back_to_the_default() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut appearance = Appearance {
            scrim: invalid,
            ..Appearance::default()
        };
        appearance.validate();
        assert_eq!(appearance.scrim, Appearance::default().scrim);
    }
}

// --- The whole thing ---------------------------------------------------------

/// Validating brings the schema back to today's: a file re-read from an
/// earlier version is rewritten at the current version.
#[test]
fn validating_brings_the_schema_up_to_date() {
    let mut settings = Settings {
        schema: 0,
        ..Settings::default()
    };
    settings.validate();
    assert_eq!(settings.schema, crate::types::SCHEMA);
}

/// The defaults are already valid: validating must not change anything in a
/// fresh configuration, or the first save would write something other than
/// what the screen showed.
#[test]
fn the_defaults_are_already_valid() {
    let mut settings = Settings::default();
    let before = settings.clone();
    settings.validate();
    assert_eq!(settings, before);
}

/// The `fullscreen` key of `options.txt` is DERIVED from the mode, and not
/// a separate field. That's what keeps the two from diverging — and they
/// would: F11 toggles this key mid-session and the game persists it, so an
/// independent field would have made fullscreen a one-way switch.
#[test]
fn fullscreen_is_derived_from_the_mode() {
    for (mode, expected) in [
        (WindowMode::Windowed, false),
        (WindowMode::Maximized, false),
        (WindowMode::Fullscreen, true),
    ] {
        let window = Window {
            mode,
            ..Window::default()
        };
        assert_eq!(window.fullscreen(), expected, "{mode:?}");
    }
}

/// The default is ABOVE the floor, and it must stay that way.
///
/// A default under the floor would get raised by `validate` on the very
/// first save: the player would see their value change on its own, with
/// nothing explaining it. And a default EQUAL to the floor would leave no
/// room to brighten — the slider would exist without serving that
/// direction. The floor is zero; the default must therefore stay strictly
/// positive.
#[test]
fn the_default_leaves_room_above_the_floor() {
    assert!(
        Appearance::default().scrim > SCRIM_FLOOR,
        "default {} against floor {SCRIM_FLOOR}",
        Appearance::default().scrim
    );
}

/// The floor fits in 0..=1, and it stays a scrim — not a wall.
///
/// Beyond roughly two thirds, the background image can no longer be told
/// apart from a flat fill: the floor would then have removed the very
/// feature it's supposed to keep readable. The low bound is no longer
/// forbidden from being zero — it's even the value it carries — but it
/// can't be negative, on pain of a negative opacity going into the CSS.
#[test]
fn the_floor_remains_a_scrim() {
    // In a `const` block: both comparisons bear on a constant, and clippy is
    // right to say so. Evaluating them at compile time changes nothing about
    // what they check — the value is re-read on every build — and states the
    // intent more accurately: this isn't a behavior being exercised, it's a
    // bound forbidden from being crossed.
    const {
        assert!(SCRIM_FLOOR >= 0.0, "a negative opacity makes no sense");
        assert!(
            SCRIM_FLOOR <= 0.66,
            "beyond that, the background image is no longer visible: might as well not have one"
        );
    }
}

// --- The clamping function, tested on its own -------------------------------

/// `clamp` at the EXACT bounds, and on either side.
///
/// This file's tests exercised each setting, but none exercised the
/// function that clamps them all: its two comparisons survived mutation,
/// `<` becoming `<=` and `>` becoming `>=` without a test flinching. The
/// symptom would have been silent — a value AT the bound clamped to the
/// bound, i.e. nothing visible — until the day the bound changed direction.
#[test]
fn clamp_returns_the_value_at_the_exact_bound() {
    use super::clamp;

    // Inside: nothing moves.
    assert_eq!(clamp(5, (1, 10)), 5);
    // AT the bounds: nothing moves either — that's what `<` and `>` say,
    // and what `<=` and `>=` would say otherwise.
    assert_eq!(clamp(1, (1, 10)), 1);
    assert_eq!(clamp(10, (1, 10)), 10);
    // Just beyond, on both sides.
    assert_eq!(clamp(0, (1, 10)), 1);
    assert_eq!(clamp(11, (1, 10)), 10);
}

/// And on floats too, since the scrim is one.
#[test]
fn clamp_also_works_for_floats() {
    use super::clamp;

    assert_eq!(clamp(0.44_f32, (0.44, 1.0)), 0.44);
    assert_eq!(clamp(1.0_f32, (0.44, 1.0)), 1.0);
    assert_eq!(clamp(0.43_f32, (0.44, 1.0)), 0.44);
    assert_eq!(clamp(1.01_f32, (0.44, 1.0)), 1.0);
}
