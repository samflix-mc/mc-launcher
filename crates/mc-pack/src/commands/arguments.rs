//! The command line, as the help page describes it.

use anyhow::{Result, bail};
use std::path::PathBuf;

/// What the command line asked for.
///
/// All options are read before anything gets built: `--data` moves the data
/// root, which the cache location of a remote pack depends on.
#[derive(Debug)]
pub struct Arguments {
    pub command: String,
    pub source_arg: Option<String>,
    pub options: mc_pack::Options,
    pub deep: bool,
    pub incident_test: bool,
    pub username: Option<String>,
    pub server: Option<String>,
    pub memory: Option<u32>,
    pub show: bool,
}

impl Arguments {
    /// Returns `None` when no command is given: the caller then shows help.
    pub fn read(args: impl Iterator<Item = String>) -> Result<Option<Arguments>> {
        let args: Vec<String> = args.collect();
        let Some(command) = args.first().cloned() else {
            return Ok(None);
        };

        let mut parsed = Arguments {
            command,
            source_arg: None,
            options: mc_pack::Options::default(),
            deep: false,
            incident_test: false,
            username: None,
            server: None,
            memory: None,
            show: false,
        };

        let mut rest = args[1..].iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--incident-test" => parsed.incident_test = true,
                "--locked" => parsed.options.locked = true,
                "--with-server" => parsed.options.with_server = true,
                "--deep" => parsed.deep = true,
                "--username" => parsed.username = rest.next().cloned(),
                "--server" => parsed.server = rest.next().cloned(),
                "--memory" => parsed.memory = rest.next().and_then(|v| v.parse().ok()),
                "--show" => parsed.show = true,
                "--instance" => parsed.options.instance_name = rest.next().cloned(),
                "--data" => {
                    let Some(dir) = rest.next() else {
                        bail!("--data expects a directory");
                    };
                    parsed.options.layout = mc_instance::Layout::new(PathBuf::from(dir));
                }
                other if other.starts_with("--") => bail!("unknown option: {other}"),
                path => parsed.source_arg = Some(path.to_string()),
            }
        }
        Ok(Some(parsed))
    }
}

#[cfg(test)]
#[path = "arguments.test.rs"]
mod tests;
