use super::{filter, layer};

/// The default must stay talkative at `info` and quiet about the network
/// libraries: without this, a single HTTP request drowns out the report.
#[test]
fn without_rust_log_the_console_talks_at_info() {
    let directives = filter(None).to_string();
    assert!(directives.contains("info"), "directives: {directives}");
    assert!(directives.contains("hyper"), "directives: {directives}");
}

/// Copying the example ".env" as-is sets an empty RUST_LOG. Treated as a
/// directive, it would mute the console — default included — with nothing
/// to explain it.
#[test]
fn an_empty_rust_log_is_as_good_as_an_absent_one() {
    let default = filter(None).to_string();
    assert_eq!(filter(Some("")).to_string(), default);
    assert_eq!(filter(Some("   ")).to_string(), default);
}

#[test]
fn a_readable_rust_log_is_honored() {
    let directives = filter(Some("trace")).to_string();
    assert!(directives.contains("trace"), "directives: {directives}");
}

/// A malformed RUST_LOG falls back to the default instead of cutting
/// everything off, and says so on standard error: the subscriber isn't set
/// up yet, that's the only channel available.
#[test]
fn an_unreadable_rust_log_falls_back_to_the_default() {
    assert_eq!(filter(Some("=====")).to_string(), filter(None).to_string());
}

#[test]
fn the_console_layer_builds() {
    // It can't be inspected — tracing-subscriber's types are opaque once
    // boxed. What the test checks is that assembling the format, the
    // redacting writer and the filter doesn't panic.
    let _layer = layer();
}
