//! Sentry: incidents alone, panics and errors.
//!
//! An automatic report saves having to ask a player to reproduce a bug
//! they've already hit. Everything that goes out this way is scrubbed
//! twice: by the emitting layer, and by the client's filters.

mod client;
mod fixture;
mod game;
mod layer;
mod scrub;

pub(crate) use client::init_sentry;
pub use fixture::send_test_event;
pub use game::capture_game_crash;
pub(crate) use layer::layer;

/// Launcher's Sentry project.
///
/// A DSN isn't a secret: it only allows writing events, and every desktop
/// client embeds its own. It stays replaceable via `SENTRY_DSN`, which
/// lets incidents from a given deployment be routed elsewhere.
const DEFAULT_DSN: &str = "https://5c97a3f2d24e9faf2a5f099a8c5a3a80@o4504715328552960.ingest.us.sentry.io/4512093170434048";

/// Is incident reporting active in this run?
pub fn telemetry_active() -> bool {
    dsn().is_some()
}

/// Waits for the incident queue to flush, and says whether it did.
///
/// [`Guard`] does the same on shutdown, but with the short budget that
/// ordinary commands can afford. The two callers that ask for more share
/// the same reason: they announce an identifier to the player, and
/// "logged" doesn't mean "sent". Searching the dashboard for an
/// identifier that a network outage kept from arriving costs more time
/// than the wait it was saving.
///
/// Returns `false` when telemetry is off: nothing was sent, since nothing
/// was emitted.
pub fn flush_incidents(budget: std::time::Duration) -> bool {
    sentry::Hub::current()
        .client()
        .map(|client| client.flush(Some(budget)))
        .unwrap_or(false)
}

/// Is incident reporting allowed?
///
/// Explicit opt-out: telemetry you can't turn off isn't telemetry, it's
/// surveillance.
fn telemetry_enabled() -> bool {
    match std::env::var("SAMFLIX_TELEMETRY") {
        Ok(v) => !matches!(v.trim(), "0" | "off" | "false" | "no" | "non"),
        Err(_) => true,
    }
}

fn dsn() -> Option<String> {
    if !telemetry_enabled() {
        return None;
    }
    let configured = std::env::var("SENTRY_DSN").unwrap_or_else(|_| DEFAULT_DSN.to_string());
    let configured = configured.trim().to_string();
    (!configured.is_empty()).then_some(configured)
}

#[cfg(test)]
#[path = "incidents.test.rs"]
mod tests;
