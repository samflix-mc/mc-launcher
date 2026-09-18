//! "What's wrong?" — what you run when you don't know yet.

use anyhow::Result;
use std::process::ExitCode;

/// Shows where the logs go and whether incident reporting is active.
///
/// The first thing to ask someone whose installation fails: the answer fits
/// in ten lines and says where to find the rest.
pub fn diagnostic(log: &mc_log::Guard, incident_test: bool) -> Result<ExitCode> {
    println!("Logs");
    match log.log_path() {
        Some(path) => println!("  file      : {}", path.display()),
        None => println!("  file      : unavailable (directory not writable)"),
    }
    println!("  directory : {}", mc_log::log_dir().display());
    // Same sort as when the filter is set: an empty RUST_LOG settles nothing,
    // and showing it as-is rendered a blank line here — on the command whose
    // only job is to say what's in effect.
    println!(
        "  console   : {}",
        std::env::var("RUST_LOG")
            .ok()
            .filter(|level| !level.trim().is_empty())
            .unwrap_or_else(|| "info (set RUST_LOG)".into())
    );

    println!("\nIncident reporting");
    println!(
        "  status    : {}",
        if mc_log::telemetry_active() {
            "active — disable with SAMFLIX_TELEMETRY=0"
        } else {
            "disabled"
        }
    );
    println!(
        "  version   : {}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("unknown")
    );
    // Shown with its origin: an unexpected environment traces back to its
    // source this way, without having to reread the code.
    println!(
        "  environment : {} ({})",
        mc_log::environment::current().as_str(),
        mc_log::environment::origin()
    );

    if incident_test {
        if !mc_log::telemetry_active() {
            println!("\nNothing to send: reporting is disabled.");
            return Ok(ExitCode::SUCCESS);
        }
        println!("\nSending a test incident…");
        let (id, sent) = mc_log::send_test_event();
        println!("  id        : {id}");
        if sent {
            println!("  send      : succeeded — find it in Sentry under this id");
            println!("  channels  : incident (Issues) and structured log (Logs)");
            println!("\n  In the Logs tab, on \"test log line\", a single");
            println!("  attribute survives — the other two are filtered by Sentry itself");
            println!("  and wouldn't say anything about our redaction:");
            println!("    witness_path → \"~/…\"  : before_send_log ran");
            println!("                 → \"/home/…\" : it didn't run, there's a leak");
        } else {
            println!("  send      : FAILED (queue not drained before timeout)");
            println!("              network blocked, wrong DSN, or nonexistent project");
            return Ok(ExitCode::FAILURE);
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
#[path = "diagnostic.test.rs"]
mod tests;
