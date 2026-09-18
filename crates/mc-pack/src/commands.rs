//! One command per module, as the help page lists them.

pub mod arguments;

#[cfg(test)]
mod fixtures;

pub mod diagnostic;
pub mod install;
pub mod launch;
pub mod lock;
pub mod usage;
pub mod verify;

pub use diagnostic::diagnostic;
pub use install::install;
pub use launch::launch;
pub use lock::lock;
pub use usage::usage;
pub use verify::verify;

use anyhow::Result;
use mc_pack::source::Source;
use std::process::ExitCode;

/// Dispatches to the requested command.
///
/// `verify` is the only one that distinguishes two successes: the
/// installation matches the lock, or it doesn't without the program having
/// failed for that.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    command: &str,
    source: &Source,
    options: &mc_pack::Options,
    deep: bool,
    username: Option<String>,
    server: Option<String>,
    memory: Option<u32>,
    show: bool,
) -> Result<ExitCode> {
    match command {
        "install" => install(source, options).await.map(|()| ExitCode::SUCCESS),
        "launch" => launch(source, options, username, server, memory, show)
            .await
            .map(|()| ExitCode::SUCCESS),
        "lock" => lock(source, options).await.map(|()| ExitCode::SUCCESS),
        "verify" => verify(source, options, deep),
        other => {
            eprintln!("unknown command: {other}");
            usage();
            Ok(ExitCode::from(2))
        }
    }
}

#[cfg(test)]
#[path = "commands.test.rs"]
mod tests;
