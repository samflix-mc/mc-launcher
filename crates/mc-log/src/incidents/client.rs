//! Sentry client options.

use crate::environment;
use crate::redact::redact;

use super::dsn;
use super::scrub::{scrub_event, scrub_log_attribute, scrub_value};

/// Initializes the client, or returns `None` if telemetry is off.
pub(crate) fn init_sentry(component: &str) -> Option<sentry::ClientInitGuard> {
    let dsn = dsn()?;
    let guard = sentry::init((dsn, options()));

    sentry::configure_scope(|scope| {
        scope.set_tag("component", component);
    });

    guard.is_enabled().then_some(guard)
}

/// The client's options, kept separate from opening it.
///
/// They carry the bulk of the decisions — what's sent, what's scrubbed,
/// what's withheld — and `sentry::init` opens a connection to the real
/// project. Verifying them therefore meant either sending for real, or
/// making them readable without a client: this is the second choice.
pub(super) fn options() -> sentry::ClientOptions {
    // `ClientOptions` is non-exhaustive: it's filled in field by field.
    let mut options = sentry::ClientOptions::default();
    // `release_name!()` would return the name of the calling crate — i.e.
    // "mc-log@…" for every binary, which would make it impossible to tell
    // one mc-pack version from another. The release names the whole
    // launcher; the component is carried by a separate tag.
    options.release = Some(format!("mc-launcher@{}", env!("CARGO_PKG_VERSION")).into());
    // Declared, never inferred from the build profile: see [`environment`].
    options.environment = Some(environment::current().as_str().into());
    // Never: this process holds authentication tokens.
    options.send_default_pii = false;
    // What `Guard` waits for on teardown, on every command — including
    // ones that never touch the network. Raising this budget to ten
    // seconds made an offline `verify` pay for a wait, with nothing to
    // send but its own log lines. Long waits are requested where they're
    // warranted, by [`flush_incidents`]: on the crash path, and in
    // [`send_test_event`].
    options.shutdown_timeout = std::time::Duration::from_secs(2);
    options.attach_stacktrace = true;
    // The SDK otherwise fills in the machine's hostname. On a player's
    // machine, that's an identifying piece of data that says nothing
    // about the crash: the empty string neutralizes the integration that
    // would set it.
    options.server_name = Some("".into());
    // Last resort: every outgoing text is scrubbed, including anything
    // third-party libraries might have added without our knowledge.
    options.before_send = Some(std::sync::Arc::new(|mut event| {
        scrub_event(&mut event);
        Some(event)
    }));
    options.before_breadcrumb = Some(std::sync::Arc::new(|mut crumb| {
        crumb.message = crumb.message.map(|m| redact(&m));
        for value in crumb.data.values_mut() {
            scrub_value(value);
        }
        Some(crumb)
    }));
    // Structured logs travel a separate channel: `before_send` never
    // sees them. Without this second filter, scrubbing would be bypassed
    // by the chattiest channel of all.
    options.before_send_log = Some(std::sync::Arc::new(|mut log| {
        log.body = redact(&log.body);
        // The SDK adds the server's address to every entry: on a
        // player's machine, that's the name of their own machine.
        log.attributes.remove("server.address");
        for attribute in log.attributes.values_mut() {
            scrub_log_attribute(attribute);
        }
        Some(log)
    }));

    options
}

#[cfg(test)]
#[path = "client.test.rs"]
mod tests;
