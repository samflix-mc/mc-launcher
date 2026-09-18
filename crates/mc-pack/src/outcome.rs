//! From the command line to the command that runs.

use anyhow::Result;
use std::process::ExitCode;

use mc_pack::source::{self, Source};

use crate::commands::arguments::Arguments;
use crate::commands::{self, diagnostic, usage};
use crate::log;

pub async fn run(log: &mc_log::Guard) -> Result<ExitCode> {
    let Some(args) = Arguments::read(std::env::args().skip(1))? else {
        usage();
        return Ok(ExitCode::from(2));
    };
    let Arguments {
        command,
        source_arg,
        options,
        deep,
        incident_test,
        username,
        server,
        memory,
        show,
    } = args;

    // Diagnostics don't need a manifest: it's exactly what you run when you
    // don't yet know what's going wrong.
    if command == "diagnostic" {
        return diagnostic(log, incident_test);
    }

    // The source is built after reading the arguments: `--data` can move the
    // data root, which the cache for a remote pack depends on.
    let source = Source::parse(
        source_arg.as_deref().unwrap_or(source::default_url()),
        &options.layout,
    );

    let _span = log::open(&command, &source);
    let start = std::time::Instant::now();

    let result = commands::execute(
        &command, &source, &options, deep, username, server, memory, show,
    )
    .await;

    log::conclude(&command, &result, start, log);
    result
}
