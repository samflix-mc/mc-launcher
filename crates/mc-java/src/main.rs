//! Checks that a usable Java is available, and installs one otherwise.
//!
//!     mc-java              detects a Java 21, installs one if there isn't
//!     mc-java --check      detects only, exit code 1 if absent
//!     mc-java --major 17   another major version
//!     mc-java --dir <DIR>  another runtime directory

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    // The guard lives until `main` returns so the log flushes: a
    // `std::process::exit` in the middle of `run` would leave it queued.
    let _log = mc_log::init("mc-java");

    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error:?}");
            ExitCode::FAILURE
        }
    }
}

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
struct Settings {
    major: u32,
    check_only: bool,
    dir: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        // Minecraft 1.21.1 requires Java 21: below that, the game stops on
        // `UnsupportedClassVersionError` before even showing a window.
        Settings {
            major: 21,
            check_only: false,
            dir: None,
        }
    }
}

/// Reading the arguments, kept separate from what they trigger.
fn parse(args: impl Iterator<Item = String>) -> Result<Settings> {
    let mut settings = Settings::default();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => settings.check_only = true,
            "--major" => {
                // An unreadable value is announced as an ordinary error: a
                // panic would show a stack trace where there's only a typo.
                let raw = args.next().context("--major expects an integer")?;
                settings.major = raw
                    .parse()
                    .with_context(|| format!("--major expects an integer, got \"{raw}\""))?;
            }
            "--dir" => {
                settings.dir = Some(PathBuf::from(args.next().context("--dir expects a path")?));
            }
            other => bail!("unknown option: {other}"),
        }
    }
    Ok(settings)
}

async fn run() -> Result<ExitCode> {
    let Settings {
        major,
        check_only,
        dir,
    } = parse(std::env::args().skip(1))?;

    execute(major, check_only, dir).await
}

/// What the settings trigger, kept separate from reading them.
///
/// `--check` is the only path that touches nothing: it says whether this
/// machine already has a usable Java, and that's the one a CI calls.
async fn execute(major: u32, check_only: bool, dir: Option<PathBuf>) -> Result<ExitCode> {
    let runtime_dir = dir.unwrap_or_else(mc_java::default_runtime_dir);

    if let Some(java) = mc_java::detect(major, &runtime_dir).await {
        println!(
            "Java {} found ({}) — {}",
            java.version.full,
            match java.origin {
                mc_java::Origin::Managed => "installed by the launcher",
                mc_java::Origin::System => "system runtime",
            },
            java.path.display()
        );
        return Ok(ExitCode::SUCCESS);
    }

    if check_only {
        eprintln!("No Java {major} or higher on this machine.");
        return Ok(ExitCode::FAILURE);
    }

    println!("No Java {major} detected, installing Temurin {major}…");
    let java = mc_java::install(major, &runtime_dir, None).await?;
    println!(
        "Java {} installed — {}",
        java.version.full,
        java.path.display()
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
#[path = "main.test.rs"]
mod tests;
