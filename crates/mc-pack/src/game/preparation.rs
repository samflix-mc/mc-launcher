//! Everything that needs gathering before launching a game session.

use anyhow::Result;

use super::instance::open;
use crate::lockfile::Lockfile;
use crate::source::Source;

use mc_instance::launch::{Command, QuickPlay, Session};

/// What the player has set, and that the game session must honor.
///
/// ## Why a structure of VALUES, and not the settings themselves
///
/// `mc-pack` doesn't depend on `mc-settings`, and shouldn't: the installation
/// library doesn't need to know an interface exists, and the CLI keeps its
/// own flags. It's the application that reads the settings file and fills
/// this in.
///
/// The boundary costs one extra structure; it buys that `mc-pack` stays
/// usable without a file written by the window getting involved.
#[derive(Debug, Clone, Default)]
pub struct Comfort {
    /// JVM memory, in megabytes. `None` lets the JVM decide — a quarter of
    /// the machine's memory, which isn't enough for a modpack.
    pub memory_mb: Option<u32>,
    /// Size of the game's window. `None` lets the game choose.
    pub resolution: Option<(u32, u32)>,
    /// Open in fullscreen.
    pub fullscreen: bool,
}

/// A game session ready to start.
pub struct GameSession {
    pub instance: mc_instance::Instance,
    pub lock: Lockfile,
    pub version_id: String,
    pub command: Command,
    pub session: Session,
    /// The server to join, and whether it came from the command line.
    pub target: Option<String>,
    pub explicit_request: bool,
    pub environment: mc_log::Environment,
}

pub async fn prepare(
    source: &Source,
    options: &crate::Options,
    identity: super::Identity,
    server: Option<String>,
    comfort: Comfort,
    report: std::sync::Arc<dyn crate::progress::Report>,
) -> Result<GameSession> {
    let session = super::identity::choose(identity).await?;
    let (manifest, lock, instance) = open(source, options)?;
    let layout = &options.layout;

    let version_id = mc_instance::neoforge::version_id(&lock.loader.version);

    let java = java_from_lock(&lock, layout, &report).await?;

    // Absent --server, the one the pack declares for this binary's
    // environment. The manifest is the same everywhere — it's the same
    // content image, served under three names — so it's up to the client to
    // choose, and it chooses with what CI froze into it at build time.
    let (target, explicit_request, environment) = super::target::choose(&manifest, server);

    let launch_options = launch_options(&comfort, target.clone());

    let command = mc_instance::launch::build(
        &version_id,
        &layout.shared(),
        &instance.game_dir,
        &java.path,
        &session,
        &launch_options,
    )?;

    Ok(GameSession {
        instance,
        lock,
        version_id,
        command,
        session,
        target,
        explicit_request,
        environment,
    })
}

/// The Java from the lock, not the system's: it's the one NeoForge was
/// installed with.
///
/// ## Why `detect` then `install`, and not `ensure`
///
/// `ensure` would do the same thing, but without saying which of the two
/// cases occurred — and that's exactly what needs to be known to avoid
/// lying on screen.
///
/// By the time a game session is being prepared, the cinematic is already on
/// `Phase::Ready`. `Tracker::phase` overwrites with no monotonicity guard:
/// emitting the "Java" step unconditionally would make the display of the
/// last step go BACKWARD to the fourth, on every launch, for a pack that's
/// otherwise complete. The player would see their launcher go backward for
/// no reason.
///
/// So it's only emitted in the case where something is actually going to
/// happen — a runtime to lay down, a hundred eighty megabytes to pull down —
/// and then it's an announced resumption, which makes sense.
async fn java_from_lock(
    lock: &Lockfile,
    layout: &mc_instance::Layout,
    report: &std::sync::Arc<dyn crate::progress::Report>,
) -> Result<mc_java::Java> {
    if let Some(java) = mc_java::detect(lock.java, &layout.runtime()).await {
        tracing::debug!(
            version = %java.version.full,
            "Java from the lock already present, nothing to announce"
        );
        return Ok(java);
    }

    report.step(crate::progress::Step::Java);
    report.note(&format!(
        "Java {} missing: installing before launching…",
        lock.java
    ));
    mc_java::install(
        lock.java,
        &layout.runtime(),
        Some(crate::installation::observer(report)),
    )
    .await
}

/// What the command line asks of the game itself.
///
/// Two settings, and two ways to silently lose them. Without `memory_mb`,
/// the JVM falls back to its default — a quarter of the machine's memory,
/// which isn't enough for a modpack and gives an `OutOfMemoryError` after
/// twenty minutes. Without `quick_play`, the game opens on its menu instead
/// of joining the server, and it looks as if the pack doesn't declare one.
fn launch_options(comfort: &Comfort, target: Option<String>) -> mc_instance::launch::LaunchOptions {
    mc_instance::launch::LaunchOptions {
        memory_mb: comfort.memory_mb,
        quick_play: target.map(QuickPlay::Multiplayer),
        // `resolution` activates `has_custom_resolution` in the descriptor,
        // which unlocks the conditional arguments Mojang put there.
        resolution: comfort.resolution,
        fullscreen: comfort.fullscreen,
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "preparation.test.rs"]
mod tests;
