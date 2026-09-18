//! Launch the game on the installed instance.
//!
//! Preparation and execution live in the library — `mc_pack::game` —
//! because the window calls them too. What's left here belongs to a
//! terminal: the summary before launching, and what gets written when the
//! session ends.

mod announce;

use anyhow::{Result, bail};
use mc_pack::source::Source;
use mc_pack::{Identity, game};

/// Out of scope for mutation testing: this command prepares the session —
/// session, Java, instance — then launches Minecraft. What it assembles is
/// tested piece by piece in `mc_pack::game`; what it shows is tested by
/// `announce` and [`error_lines`].
#[mutants::skip]
pub async fn launch(
    source: &Source,
    options: &mc_pack::Options,
    username: Option<String>,
    server: Option<String>,
    memory: Option<u32>,
    show: bool,
) -> Result<()> {
    // `--username` asks for an offline session, its absence asks for the
    // signed-in account. The choice is explicit on both sides: a silent
    // fallback would put a player on a server under an identity they didn't
    // choose.
    let identity = match username {
        Some(username) => Identity::Offline(username),
        None => Identity::Microsoft,
    };

    // A silent report: the command line has no bar to feed, and the
    // installation of a missing Java already gets told through the log.
    let game_session = game::prepare(
        source,
        options,
        identity,
        server,
        game::Comfort {
            memory_mb: memory,
            ..Default::default()
        },
        std::sync::Arc::new(mc_pack::Silent),
    )
    .await?;
    announce::announce(&game_session);

    // Show without launching: this is what you look at when the game
    // refuses to start, and what lets you replay the command by hand.
    if show {
        println!("{}", game_session.command.display());
        return Ok(());
    }

    let report = game::play(&game_session).await?;
    for line in error_lines(&report.errors) {
        eprintln!("{line}");
    }

    match report.outcome {
        mc_instance::launch::Outcome::Normal => Ok(()),
        mc_instance::launch::Outcome::Interrupted { signal } => {
            println!("Minecraft was interrupted (signal {signal}).");
            Ok(())
        }
        // The only case that returns a nonzero exit code: this is where the
        // command line decides, not the library.
        mc_instance::launch::Outcome::Failed { code } => bail!(
            "Minecraft stopped with code {code} — logs in {}",
            game::logs(&game_session).display()
        ),
    }
}

/// What's told to the player about the errors caught during their session.
///
/// Empty when there are none: announcing "0 errors caught" after a session
/// that went fine would send someone looking for a nonexistent problem.
/// Otherwise, each one is named — Minecraft catches a lot of them and keeps
/// going, and these are often what explains a behavior reported much later.
fn error_lines(errors: &[mc_instance::crash::Crash]) -> Vec<String> {
    if errors.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!(
        "\n{} errors caught during the session:",
        errors.len()
    )];
    lines.extend(
        errors
            .iter()
            .map(|error| format!("  {}: {}", error.exception, error.message)),
    );
    lines
}

#[cfg(test)]
#[path = "launch.test.rs"]
mod tests;
