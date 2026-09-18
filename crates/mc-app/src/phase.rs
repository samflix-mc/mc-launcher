//! The full path, as shown by the window.
//!
//! `mc_pack::Step` covers installation, and nothing else: signing in with
//! Microsoft and checking the license aren't part of it — the CLI only
//! authenticates at launch, and `mc-pack` doesn't check any license. The
//! app, on the other hand, chains all three, and must show them as one
//! piece.
//!
//! Hence this wider enum, which frames `mc-pack`'s without replacing it: two
//! phases before, two after.
//!
//! The path is displayed **in full from the start**, each phase carrying its
//! state. That's what distinguishes "we're halfway there" from "something is
//! happening": a player who sees the seven remaining steps knows what to
//! expect, whereas a single line that changes says nothing about duration.

use serde::Serialize;

/// A phase of the cinematic, from the window opening to the game launched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// Microsoft session: resumed, or opened by device code.
    #[serde(rename = "signin")]
    SignIn,
    /// Does the account own Minecraft Java Edition?
    License,
    /// Reading the pack manifest, and the lock.
    Pack,
    /// Which version of NeoForge.
    Loader,
    /// Mojang's files: client, libraries, assets.
    Minecraft,
    /// The Java runtime, detected or installed.
    Java,
    /// The NeoForge installer.
    NeoForge,
    /// Resolving, downloading and laying out the mods.
    Mods,
    /// The lock, written last.
    Lock,
    /// Everything is in place: the "Play" button lights up.
    Ready,
    /// The game is running.
    Launch,
}

impl Phase {
    /// All the phases, in the order they occur.
    pub const ALL: [Phase; 11] = [
        Phase::SignIn,
        Phase::License,
        Phase::Pack,
        Phase::Loader,
        Phase::Minecraft,
        Phase::Java,
        Phase::NeoForge,
        Phase::Mods,
        Phase::Lock,
        Phase::Ready,
        Phase::Launch,
    ];

    /// What the window writes next to the phase.
    ///
    /// Written for reading: unlike the serialized identifier, this text can
    /// change without breaking anything.
    pub fn label(self) -> &'static str {
        match self {
            Phase::SignIn => "Microsoft account",
            Phase::License => "Minecraft license",
            Phase::Pack => "Reading the pack",
            Phase::Loader => "Loader version",
            Phase::Minecraft => "Game files",
            Phase::Java => "Java",
            Phase::NeoForge => "Installing NeoForge",
            Phase::Mods => "Mods",
            Phase::Lock => "Finalizing",
            Phase::Ready => "Ready to play",
            Phase::Launch => "Game launched",
        }
    }

    /// The rank of the phase, starting from zero.
    pub fn rank(self) -> usize {
        Phase::ALL
            .iter()
            .position(|phase| *phase == self)
            .expect("every phase is in ALL")
    }
}

/// `mc-pack`'s steps take place in the middle of the path.
impl From<mc_pack::Step> for Phase {
    fn from(step: mc_pack::Step) -> Self {
        match step {
            mc_pack::Step::Pack => Phase::Pack,
            mc_pack::Step::Loader => Phase::Loader,
            mc_pack::Step::Minecraft => Phase::Minecraft,
            mc_pack::Step::Java => Phase::Java,
            mc_pack::Step::NeoForge => Phase::NeoForge,
            mc_pack::Step::Mods => Phase::Mods,
            mc_pack::Step::Lock => Phase::Lock,
        }
    }
}

#[cfg(test)]
#[path = "phase.test.rs"]
mod tests;
