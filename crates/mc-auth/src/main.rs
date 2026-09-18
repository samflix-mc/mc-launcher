//! Manages the launcher's Microsoft session, and lets it be checked.
//!
//!     mc-auth login              opens a session and saves it
//!     mc-auth whoami             shows the saved session
//!     mc-auth logout             forgets the session
//!     mc-auth --offline <NICKNAME> local profile, no Microsoft
//!
//! Signing in presents the identity of the official launcher: see the
//! crate's docs and the README for what that choice implies.

use anyhow::{Result, bail};

mod commands;

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Login,
    Whoami,
    Logout,
    Offline(String),
}

/// Reading the arguments, separate from what they trigger.
///
/// Three of the four commands contact Microsoft or touch the session file;
/// the dispatch itself checks out on its own.
fn parse(args: &[String]) -> Result<Command> {
    match args.first().map(String::as_str) {
        Some("login") => Ok(Command::Login),
        Some("whoami") => Ok(Command::Whoami),
        Some("logout") => Ok(Command::Logout),
        Some("--offline") => match args.get(1) {
            Some(nickname) => Ok(Command::Offline(nickname.clone())),
            None => bail!("usage: mc-auth --offline <NICKNAME>"),
        },
        _ => bail!("a command is required"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // This binary handles tokens: mc-log's redaction applies to everything
    // it outputs, including the file log.
    let _log = mc_log::init("mc-auth");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse(&args) {
        Ok(command) => command,
        Err(error) => {
            usage();
            return Err(error);
        }
    };

    match command {
        Command::Login => commands::login().await,
        Command::Whoami => commands::whoami().await,
        Command::Logout => commands::logout(),
        Command::Offline(nickname) => {
            commands::offline(&nickname);
            Ok(())
        }
    }
}

fn usage() {
    eprintln!("usage:");
    eprintln!("  mc-auth login              opens a session and saves it");
    eprintln!("  mc-auth whoami             shows the saved session");
    eprintln!("  mc-auth logout             forgets the session");
    eprintln!("  mc-auth --offline <NICKNAME> local profile, no Microsoft");
}

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;
