//! The states the interface has to know how to show.
//!
//! ## Why scenarios and not the real chain
//!
//! The point of this server is to work on the INTERFACE. Now, most of the
//! states it has to know how to draw are painful or impossible to trigger
//! for real: an install stuck at thirty-seven percent, a pack missing three
//! mods, an account without a Minecraft license, an offline host. Reaching
//! them for real would mean breaking something, and fixing it between two
//! tries.
//!
//! A scenario makes all of them reachable in one request. That's what makes
//! it possible to look at the "missing mods" screen without having to
//! publish a broken pack.
//!
//! **What is NOT simulated**: `brand`, `path`, `settings`,
//! `save_settings`. Those four call the REAL implementation — they touch
//! neither the network nor eight hundred megabytes, and watching them lie
//! wouldn't teach anything. `save_settings` therefore really writes to
//! `settings.json`, which is exactly what we want to put to the test: it's
//! where the bounds apply.

use serde::Serialize;

use crate::commands::{Account, StepView};
use crate::phase::Phase;
use crate::tracker::Progress;

/// The state the server serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Nobody is signed in: the screen must open on sign-in.
    SignedOut,
    /// Valid Microsoft account, but no Minecraft license. A STATE to
    /// display, not a redirect.
    NoLicense,
    /// Signed in, nothing installed: the button says INSTALL.
    NothingInstalled,
    /// Signed in, pack placed and conformant: the button says PLAY.
    ReadyToPlay,
    /// Pack placed but the published lock has changed: PLAY, with a catch-up.
    NeedsCatchUp,
    /// The pack's host didn't respond: the drift is unknown.
    Offline,
    /// An install is in progress, stuck partway through.
    Installing,
    /// The install went through, but three mods are missing.
    MissingMods,
}

impl State {
    /// All the scenarios, with the name by which they're requested.
    pub const ALL: [(&'static str, State); 8] = [
        ("signed-out", State::SignedOut),
        ("no-license", State::NoLicense),
        ("nothing-installed", State::NothingInstalled),
        ("ready-to-play", State::ReadyToPlay),
        ("needs-catch-up", State::NeedsCatchUp),
        ("offline", State::Offline),
        ("installing", State::Installing),
        ("missing-mods", State::MissingMods),
    ];

    pub fn from_name(name: &str) -> Option<State> {
        State::ALL
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, state)| *state)
    }

    /// The account, or `None` when nobody is signed in.
    pub(crate) fn account(self) -> Option<Account> {
        match self {
            State::SignedOut => None,
            State::NoLicense => Some(Account {
                username: "NoLicense".to_string(),
                uuid: "00000000-0000-0000-0000-00000000dead".to_string(),
                owns_the_game: false,
            }),
            _ => Some(Account {
                username: "thesam1798".to_string(),
                uuid: "9f6e4a0c-1b2d-4e3f-8a9b-0c1d2e3f4a5b".to_string(),
                owns_the_game: true,
            }),
        }
    }

    /// What the disk and the published pack say.
    pub(crate) fn pack(self) -> mc_pack::PackState {
        use mc_pack::comparison::{Action, Drift};

        let (action, drift, installed, offline) = match self {
            State::SignedOut | State::NoLicense | State::NothingInstalled => {
                (Action::Install, Drift::Absent, false, false)
            }
            State::ReadyToPlay | State::MissingMods => (Action::Play, Drift::UpToDate, true, false),
            State::NeedsCatchUp | State::Installing => (Action::Play, Drift::Update, true, false),
            State::Offline => (Action::Play, Drift::Unknown, true, true),
        };

        mc_pack::PackState {
            action,
            drift,
            offline,
            installed,
            name: Some("samflix".to_string()),
            version: Some("0.1.0".to_string()),
            java: Some(21),
            mods: 128,
            generation: 0,
        }
    }

    /// The progress shown on opening.
    ///
    /// This is what distinguishes "nothing's happening" from "an install is
    /// in progress": the interface has to know how to draw both, and we
    /// can't wait for a real install to reach thirty-seven percent to see
    /// what it looks like.
    pub(crate) fn progress(self) -> Progress {
        match self {
            State::Installing => Progress {
                phase: Phase::Mods,
                done: false,
                note: Some("128 mods, 43 of them added by dependency".to_string()),
                file: Some("sodium-neoforge-0.6.13.jar".to_string()),
                bytes: 312_000_000,
                total: 840_000_000,
                files: 47,
                files_total: 128,
                active: true,
                rate: 8_400_000,
                remaining: Some(63),
            },
            State::SignedOut | State::NoLicense => Progress {
                phase: Phase::SignIn,
                done: false,
                ..empty()
            },
            State::ReadyToPlay | State::NeedsCatchUp | State::Offline | State::MissingMods => {
                Progress {
                    phase: Phase::Ready,
                    done: true,
                    ..empty()
                }
            }
            State::NothingInstalled => Progress {
                phase: Phase::License,
                done: true,
                ..empty()
            },
        }
    }

    /// What a session returns, once the game is closed.
    pub(crate) fn play_result(self, played: bool) -> crate::commands::pack::Report {
        crate::commands::pack::Report {
            verdict: if played {
                "Session finished.".to_string()
            } else if matches!(self, State::NeedsCatchUp) {
                "The pack is installed.".to_string()
            } else {
                "The pack was already up to date: nothing to place.".to_string()
            },
            caught_up: matches!(self, State::NeedsCatchUp),
            missing: match self {
                State::MissingMods => vec![
                    "journeymap".to_string(),
                    "waystones".to_string(),
                    "litematica".to_string(),
                ],
                _ => Vec::new(),
            },
            drifts: match self {
                State::NeedsCatchUp => vec!["jei: 19.21.1.317 instead of 19.21.0.247".to_string()],
                _ => Vec::new(),
            },
            offline: matches!(self, State::Offline),
            purge: Vec::new(),
        }
    }

    /// The news feed.
    ///
    /// Non-empty in every scenario except `offline`: that's the page you
    /// CAN'T look at today, since the three hosts return 404. Without these
    /// posts, it would stay invisible until mc-content has published.
    pub(crate) fn feed(self) -> mc_news::Feed {
        let posts = if matches!(self, State::Offline) {
            Vec::new()
        } else {
            demo_posts()
        };

        mc_news::Feed {
            posts,
            offline: matches!(self, State::Offline),
            discarded: Vec::new(),
        }
    }
}

fn empty() -> Progress {
    Progress {
        phase: Phase::SignIn,
        done: false,
        note: None,
        file: None,
        bytes: 0,
        total: 0,
        files: 0,
        files_total: 0,
        active: false,
        rate: 0,
        remaining: None,
    }
}

/// Two posts, one of them illustrated, and one that exercises the whole
/// recognized markdown.
///
/// The second one exists so the page gets looked at with REAL content:
/// headings, lists, bold, code, link, separator. A one-paragraph news page
/// doesn't show its formatting flaws.
fn demo_posts() -> Vec<mc_news::Post> {
    let parse = |body: &str| mc_news::parsing::parse(body, "https://example.invalid/");

    vec![
        mc_news::Post {
            id: "2026-09-launch".to_string(),
            title: "The launcher is here".to_string(),
            date: "2026-09-18T18:00:00Z".to_string(),
            pinned: true,
            image: None,
            body: parse(
                "The launcher installs the pack and launches the game **in a single \
                 step**.\n\n\
                 What you need to know:\n\n\
                 - it installs what the server is running, down to the version\n\
                 - it places the Java it needs, without touching the system's\n\
                 - it never reinstalls anything as long as the pack hasn't moved\n\n\
                 Settings are in `Settings`.",
            ),
        },
        mc_news::Post {
            id: "2026-09-rules".to_string(),
            title: "Server rules".to_string(),
            date: "2026-09-17T12:00:00Z".to_string(),
            pinned: false,
            image: None,
            body: parse(
                "Three rules, and each one fits on a line.\n\n\
                 ## Respect\n\n\
                 No insults, no harassment. It's the only one that leads to an \
                 immediate ban.\n\n\
                 ## Builds\n\n\
                 Don't break other people's stuff. A misplaced `/back` isn't an \
                 excuse.\n\n\
                 ---\n\n\
                 Details are on [the rules page](https://example.invalid/rules).",
            ),
        },
    ]
}

/// What the server announces about itself, at the root.
#[derive(Serialize)]
pub(crate) struct Home {
    pub(crate) scenario: String,
    pub(crate) scenarios: Vec<String>,
    pub(crate) commands: Vec<String>,
}

/// The full path of phases, as the real command renders it.
pub(crate) fn path() -> Vec<StepView> {
    crate::commands::path()
}
