//! What an installation tells us while it's working.
//!
//! The final report only serves the one who waited until the end. Between the
//! first byte and them, several minutes go by, and someone is watching.
//!
//! ## Why a step, and not just a line of text
//!
//! The first version of this channel was `&dyn Fn(&str)`: the installation
//! pushed already-formatted sentences, and the command line printed them.
//! That's enough for a terminal, which stacks lines. It isn't enough for a
//! window, which has to know *where things stand* — which step is done, which
//! is working, which remain — in order to draw something other than a
//! scrolling log. Reconstructing that by re-reading sentences would amount to
//! parsing your own output, and the slightest rewording would break the
//! display without breaking the build.
//!
//! The two therefore coexist, and don't say the same thing: [`Report::step`]
//! carries the structure, [`Report::note`] the detail only a human reads.
//!
//! ## And the download
//!
//! [`Report::download`] receives what `mc-dl` emits — bytes, file in
//! progress, announced batches. `mc-pack` just plugs in the wire: it puts the
//! observer on its HTTP client and on the mods' one, and doesn't look at what
//! flows through it.

/// Where an installation stands.
///
/// The declaration order is the order of the steps, and [`Step::ALL`] depends
/// on it: it's what lets a display show the whole path from the start, rather
/// than discovering the steps one by one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    /// Reading the manifest, and the lockfile if there is one.
    Pack,
    /// Which NeoForge version — resolved before everything else, so the
    /// lockfile records a number and not the word "latest".
    Loader,
    /// Mojang's files: the client, its libraries, its assets.
    Minecraft,
    /// The Java runtime, detected or installed.
    Java,
    /// The NeoForge installer, which patches the vanilla client.
    NeoForge,
    /// Resolving, downloading and distributing the mods.
    Mods,
    /// The lockfile, written last: it describes what was actually done.
    Lock,
}

impl Step {
    /// All the steps, in the order they occur.
    pub const ALL: [Step; 7] = [
        Step::Pack,
        Step::Loader,
        Step::Minecraft,
        Step::Java,
        Step::NeoForge,
        Step::Mods,
        Step::Lock,
    ];

    /// A stable identifier, meant to be read by a machine.
    ///
    /// Not a label: what's displayed gets translated and reworded, what's
    /// written here serves as a key and must not move.
    pub fn as_str(self) -> &'static str {
        match self {
            Step::Pack => "pack",
            Step::Loader => "loader",
            Step::Minecraft => "minecraft",
            Step::Java => "java",
            Step::NeoForge => "neoforge",
            Step::Mods => "mods",
            Step::Lock => "lock",
        }
    }

    /// The step's rank in the sequence, starting at zero.
    pub fn rank(self) -> usize {
        Step::ALL
            .iter()
            .position(|step| *step == self)
            .expect("every step is in ALL")
    }
}

/// Who the installation reports to.
///
/// `Send + Sync + 'static` because the download observer that `mc-pack`
/// derives from it crosses `mc-dl`'s concurrent tasks.
pub trait Report: Send + Sync + 'static {
    /// A step begins.
    fn step(&self, step: Step);

    /// A detail line, meant to be read as is.
    fn note(&self, text: &str);

    /// A download makes progress.
    ///
    /// Ignored by default: a terminal does nothing with it, and the rate
    /// scrolls by faster than it can be read.
    /// Out of reach of mutation tests: all this method does is write to
    /// standard output, which Rust can't read back from the process that
    /// emits it. What it COMPUTES — the rate, the time remaining, the
    /// percentage — is produced by `Tracker`, which is tested on its own.
    #[mutants::skip]
    fn download(&self, progress: mc_dl::Progress<'_>) {
        let _ = progress;
    }

    /// Mod resolution makes progress.
    ///
    /// **Distinct from [`Report::download`], and that's the whole point.**
    /// Resolution spends most of its time querying APIs — some thirty seconds
    /// for a fifty-mod pack — for responses of a few kilobytes. No byte bar
    /// moves during that time, and the screen is indistinguishable from a
    /// frozen one.
    ///
    /// `total` is an estimate that can grow: a resolved request can spawn
    /// others.
    ///
    /// Ignored by default, like the download: a terminal does nothing with
    /// it.
    #[mutants::skip]
    fn resolution(&self, done: usize, total: usize) {
        let _ = (done, total);
    }

    /// The game has just started, under this process number.
    ///
    /// Two uses, and that's why the number comes with the announcement. The
    /// window has to say the game is running — it used to display
    /// "Installing…" for the whole game session, for lack of anything telling
    /// it otherwise. And it has to be able to stop it: a Minecraft frozen on a
    /// loading screen never gives back control.
    ///
    /// Ignored by default: a terminal has no use for it, whoever launched the
    /// command already has their hand on the process.
    #[mutants::skip]
    fn game_session_started(&self, pid: u32) {
        let _ = pid;
    }
}

/// A report that listens to nothing.
///
/// For callers that only want the result — tests, and any use where nobody's
/// watching.
pub struct Silent;

impl Report for Silent {
    fn step(&self, _step: Step) {}
    fn note(&self, _text: &str) {}
}

#[cfg(test)]
#[path = "progress.test.rs"]
mod tests;
