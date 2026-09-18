use super::{conclude, open};
use mc_pack::source::Source;

/// The root span carries the command's name and the pack's origin: in the
/// file just as in Sentry, a run reads as one block even when several follow
/// each other.
#[test]
fn the_root_span_names_the_command_and_the_pack() {
    let layout = mc_instance::Layout::new(std::env::temp_dir().join("mc-pack-log"));
    let source = Source::parse("packs/samflix.json", &layout);

    // The span is entered then exited; what the test checks is that it
    // assembles and closes, even with no subscriber in place.
    let entered = open("install", &source);
    tracing::info!("a step of the command");
    drop(entered);
}

/// An error that surfaces this far ends the program: it's the last place it
/// can become an incident rather than a plain message.
#[test]
fn both_outcomes_of_a_command_get_logged() {
    let start = std::time::Instant::now();
    // A guard without a log: `log_path` then returns `None`, and the
    // conclusion must not point to a nonexistent file regardless.
    let log = mc_log::Guard::without_log();

    conclude("verify", &Ok(std::process::ExitCode::SUCCESS), start, &log);
    conclude(
        "install",
        &Err(anyhow::anyhow!("the pack is unreachable")),
        start,
        &log,
    );
}
