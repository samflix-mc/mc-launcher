//! Setting up the three destinations.

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::guard::{Guard, current_log_name};
use crate::redact::redact;
use crate::{BoxedLayer, console, file, incidents};

/// Sets up logging for a given component.
///
/// `component` names the binary — it prefixes the log file and tags
/// incidents, which makes it possible to tell an install failure apart from
/// an auth failure without opening the report.
pub fn init(component: &str) -> Guard {
    // Before Sentry: it chains its handler on top of the existing one.
    // Set after, ours would replace it and no panic would ever be reported
    // again.
    install_panic_hook();
    let sentry_guard = incidents::init_sentry(component);
    let (file_layer, file_guard, log_dir) = file::file_layer(component);

    let mut layers: Vec<BoxedLayer> = Vec::new();
    layers.push(console::layer());
    if let Some(layer) = file_layer {
        layers.push(layer);
    }
    if sentry_guard.is_some() {
        layers.push(incidents::layer());
    }

    tracing_subscriber::registry().with(layers).init();

    let log = log_dir.map(|dir| (dir, component.to_string()));
    if let Some((dir, component)) = &log {
        tracing::debug!(
            file = %dir.join(current_log_name(component)).display(),
            "log opened"
        );
    }

    Guard::new(file_guard, sentry_guard, log)
}

/// Replaces the default display of a panic.
///
/// Rust's standard handler writes the panic message directly to standard
/// error, without going through `tracing`: it therefore escapes both
/// redaction and the log file. Two consequences, both observed before
/// writing this — a token present in a panic message showed up in the clear
/// in the terminal, and the panic stayed absent from the very file players
/// are asked to attach.
///
/// The message goes out at `warn` and not `error`: the incident itself is
/// reported by Sentry's handler, with its full call trace, and an `error`
/// here would surface it a second time.
fn install_panic_hook() {
    // The original handler is deliberately not called back: it would
    // rewrite the unredacted message to standard error, which is precisely
    // what was just avoided.
    std::panic::set_hook(Box::new(move |info| {
        let message = redact(&info.to_string());
        eprintln!("\n{message}");
        eprintln!("(rerun with RUST_BACKTRACE=1 for the call trace)");
        tracing::warn!(panic = %message, "the program stopped on a panic");
    }));
}

#[cfg(test)]
#[path = "init.test.rs"]
mod tests;
