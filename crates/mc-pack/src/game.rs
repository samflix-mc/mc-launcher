//! Launch a game session on top of what's installed.
//!
//! The counterpart to [`install`](crate::install): one lays down the pack, the
//! other plays it. The two stay distinct gestures — see `docs/lancement.md`:
//! chaining them would make anyone who just wanted to play wait on eight
//! hundred megabytes.
//!
//! ## Why it's here and not in a binary
//!
//! This module used to live in `mc-pack` **the binary**, under
//! `commandes/lancement/`. As long as there was only one command line, that
//! cost nothing. As soon as a window wanted to launch the game, it couldn't
//! call it: it had to reassemble the game session, the lock's Java, and the
//! command line itself. Two assemblies for the same thing, only one of which
//! verifies the instance's coherence — exactly the divergence we don't want.
//!
//! So it lives in the library, and both callers use it. What's left to the
//! binary is what truly belongs to it: display.
//!
//! ## What preparation checks before launching
//!
//! The **local** pack, never today's: fetching the published pack would
//! describe mods the folder doesn't contain, and would forbid playing without
//! a network. The Java **from the lock**, not the system's: it's the one
//! NeoForge was installed with. And the instance's coherence, without which
//! the server settles it with an ejection that doesn't name its cause.

mod coherence;
mod execution;
mod identity;
pub mod incidents;
mod instance;
mod preparation;
mod sequence;
mod target;

pub use execution::{logs, play, play_announced};
pub use identity::Identity;
pub use preparation::{Comfort, GameSession, prepare};
pub use sequence::{UpdateOutcome, should_catch_up, update, update_and_play};
