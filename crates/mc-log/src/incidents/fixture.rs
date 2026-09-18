//! "Does this actually get reported?"

use super::flush_incidents;

/// Sends a test incident and returns its identifier.
///
/// Answers "does this actually get reported?" without having to trigger
/// a real panic. The returned identifier is the one to look up in the
/// dashboard: if the two match, the whole chain — sending, network,
/// project, scrubbing — is verified.
pub fn send_test_event() -> (sentry::types::Uuid, bool) {
    // Both channels take different routes through different filters:
    // testing them together avoids believing one works because the
    // other does.
    // Verifying that scrubbing ran requires a marker only ours produces.
    //
    // Two attempts failed on this point. An attribute containing
    // "access_token" comes back "[Filtered]": that's Sentry's own
    // server-side filtering, which recognizes the keyword. A bare JWT
    // also comes back "[Filtered]": Sentry recognizes the shape too. In
    // both cases the test passed without saying anything about
    // `before_send_log`, since the result would have been the same had
    // our filter not run at all.
    //
    // The home directory path, though, is a secret to no one: no server
    // rule touches it. Our filter reduces it to "~". That rewrite can
    // only come from us.
    let witness = std::env::var("HOME").unwrap_or_else(|_| "/home/user".into());

    tracing::info!(
        channel = "structured logs",
        component = "mc-log",
        // These two show defense in depth, without proving anything.
        with_keyword = "access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA",
        bare_token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdEp3dE51",
        // This one is decisive: "~/…" proves before_send_log ran, the
        // full path proves it didn't.
        path_witness = %format!("{witness}/.local/share/samflix-mc"),
        "test log line"
    );

    let id = sentry::capture_message(
        "test incident sent by mc-pack diagnostic --incident-test",
        sentry::Level::Info,
    );
    // Sending is asynchronous: without this wait, the process would exit
    // before the request goes out. The return says whether the queue
    // flushed — the difference between "an identifier was drawn" and
    // "the incident actually went out".
    (id, flush_incidents(std::time::Duration::from_secs(10)))
}

#[cfg(test)]
#[path = "fixture.test.rs"]
mod tests;
