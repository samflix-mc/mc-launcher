//! What the launcher knows about the installation laid down on THIS machine.
//!
//! ## The question this module answers
//!
//! The lockfile says what the pack SHOULD be. It doesn't say what's actually
//! laid down on this disk, or what happened here last time. Until now, the
//! launcher didn't know: `State.installation` was only populated by an
//! installation done in the current session, so at startup a perfectly
//! installed pack looked absent.
//!
//! A file next to the instance is enough, and that's this module.
//!
//! ## Why it checks its own `schema`
//!
//! The lockfile carries a `schema` field that's written and never read back —
//! the only `schema` read anywhere in this repo is the manifest's. Repeating
//! that dead field in a new file would be deliberately making the mistake
//! we're calling out.
//!
//! So it's read back, and a refusal is treated as an ABSENCE: generation 0,
//! hence a purge, hence a clean reinstall. That's the safe behavior — a state
//! file we can't read must not prevent playing, and it must not pretend we
//! know the installation either.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Version of the local state format.
pub const SCHEMA: u32 = 1;

/// The pack's name when we haven't read it yet.
///
/// Used for the one case where the published lockfile is unreachable: we
/// still have to query an instance directory to know whether something's
/// laid down, and we then have no source giving us the name. It's the one
/// mc-content has always published.
pub const DEFAULT_NAME: &str = "samflix";

/// What we kept from the last successful installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalState {
    pub schema: u32,
    /// The lockfile's digest as it was at the end of the installation.
    ///
    /// It's this one that gets compared against the published lockfile's
    /// digest to decide whether there's anything to do. On the canonical
    /// form of both sides: see `Lockfile::digest`.
    pub lock_sha512: String,
    /// The generation under which this installation was laid down.
    pub generation: u32,
    /// When, in UTC. For diagnostics only — nothing decides based on it.
    pub placed_at: String,
}

/// Where an instance's state is stored.
///
/// Next to the game directory and not inside it: what's in `minecraft/`
/// belongs to the game, and a foreign file there would get picked up by a
/// tool that syncs instances.
pub fn path(instance: &mc_instance::Instance) -> PathBuf {
    instance.dir.join("state.json")
}

impl LocalState {
    /// The state of an installation we just finished.
    pub fn new(lock_sha512: String, generation: u32, placed_at: String) -> Self {
        Self {
            schema: SCHEMA,
            lock_sha512,
            generation,
            placed_at,
        }
    }

    /// Reads the state back, or returns `None`.
    ///
    /// **Never returns an error.** An absent state, an unreadable one, or one
    /// with an unknown schema all mean the same thing: "we don't know what's
    /// laid down". Distinguishing these cases would force the caller to
    /// decide what to do with a distinction it can't act on — and the only
    /// safe behavior is the same in all three cases.
    pub fn read(path: &Path) -> Option<Self> {
        let raw = std::fs::read(path).ok()?;
        match serde_json::from_slice::<Self>(&raw) {
            Ok(state) if state.schema == SCHEMA => Some(state),
            Ok(state) => {
                tracing::warn!(
                    schema = state.schema,
                    expected = SCHEMA,
                    file = %path.display(),
                    "unknown local state schema: treated as absent"
                );
                None
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    file = %path.display(),
                    "unreadable local state: treated as absent"
                );
                None
            }
        }
    }

    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        mc_dl::write_atomic(path, json.as_bytes())
    }
}

/// What to do before installing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Before {
    /// Install by diff, as usual.
    Differential,
    /// Erase what the launcher has laid down, then install.
    Purge,
}

/// Should we purge before installing?
///
/// A PURE function, and that's the whole point: the rule reads in four lines
/// and is tested without touching disk.
///
/// ## The four cases, and why it's `<` and not `!=`
///
/// - **No state**: we know nothing of what's laid down. Purge. This is the
///   case for every machine already installed, the first time it launches
///   this version — one hundred percent of the population, once.
/// - **Equal generation**: nothing special, differential.
/// - **Requested generation HIGHER**: whoever published asked for a clean
///   reinstall. Purge.
/// - **Requested generation LOWER**: a rollback of the published pack. We do
///   NOT purge. The `<` is here rather than a `!=` for this exact reason:
///   going back to an earlier generation means republishing a state that was
///   known good, and erasing the installation on that occasion would punish
///   the player for an operations decision. The diff will put back the
///   earlier files.
pub fn decide(placed: Option<&LocalState>, requested_generation: u32) -> Before {
    match placed {
        None => Before::Purge,
        Some(state) if state.generation < requested_generation => Before::Purge,
        Some(_) => Before::Differential,
    }
}

/// What the purge erased, for the report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Purge {
    /// The directories actually cleared, relative to the game directory.
    pub cleared: Vec<String>,
    /// What couldn't be erased, with the reason.
    pub failures: Vec<String>,
}

impl Purge {
    pub fn happened(&self) -> bool {
        !self.cleared.is_empty() || !self.failures.is_empty()
    }
}

/// What the launcher is allowed to erase — and NOTHING else.
///
/// ## An allow list, and why it's vital
///
/// A DENY list — "erase everything except `saves` and `options.txt`" — would
/// be the right way to lose a world the day a mod stores its data in a
/// directory nobody thought of. Server logs, screenshots, Litematica
/// schematics, Waystones notebooks: all of that lives in the game directory,
/// and nothing tells it apart from leftover cruft.
///
/// So we name what we erase. These four directories share one property:
/// their content is ENTIRELY relaid by the installation that follows, and the
/// player never puts anything there they'd care to keep.
///
/// `config/` is deliberately NOT part of it, and that's the point that needs
/// the most care: it's where the player tunes their mods, and a lost setting
/// is a whole evening of reconfiguration. A configuration format change that
/// truly required clearing `config/` calls for a human decision, announced to
/// players — not an incremented number in a file.
const ERASABLE: &[&str] = &["mods", "shaderpacks", "resourcepacks", "libraries"];

/// Erases what the launcher laid down, and nothing more.
///
/// Takes the GAME directory — the one holding `mods`, `saves`,
/// `options.txt` — and only clears the directories in [`ERASABLE`].
///
/// Never returns an error: a directory that can't be erased — a file locked
/// by an antivirus, a mount point — must not stop the installation from
/// continuing. It will overwrite on top, and the report will say what
/// resisted.
pub fn purge(game_dir: &Path) -> Purge {
    let mut purge = Purge::default();

    for name in ERASABLE {
        let target = game_dir.join(name);
        if !target.exists() {
            continue;
        }
        match std::fs::remove_dir_all(&target) {
            Ok(()) => {
                tracing::info!(directory = %target.display(), "purged");
                purge.cleared.push((*name).to_string());
            }
            Err(error) => {
                tracing::warn!(
                    directory = %target.display(),
                    error = %error,
                    "purge failed: the install will overwrite it"
                );
                purge.failures.push(format!("{name}: {error}"));
            }
        }
    }

    purge
}

#[cfg(test)]
#[path = "state.test.rs"]
mod tests;
