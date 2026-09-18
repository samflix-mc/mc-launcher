//! Logging and incident reporting.
//!
//! Three destinations, three distinct purposes:
//!
//! - **the console** — what a user sees while waiting. Quiet by default
//!   (`info`), tunable via `RUST_LOG`;
//! - **a file** — the full detail (`debug`), with timestamp and target.
//!   It's what we ask a player to attach when something went wrong, and
//!   it's written even when the console is swallowed by a graphical
//!   interface;
//! - **Sentry** — incidents alone: panics and errors. An automatic report
//!   avoids having to ask a player to reproduce a bug they already hit.
//!
//! ## Which level for what
//!
//! Without a rule, levels drift: everything ends up at `info` and the file
//! becomes unreadable, or everything ends up at `debug` and the console
//! stops saying anything. The rule fits in one question — **who needs to
//! read this line?**
//!
//! | level | who reads it | examples |
//! |---|---|---|
//! | `error` | the user, right away | the install failed, a dependency is missing |
//! | `warn` | whoever diagnoses afterward | a network retry, a rejected key, a source fallback |
//! | `info` | the report of the run | milestones: version resolved, 7 mods retained, instance installed |
//! | `debug` | whoever is looking for why | every mod retained, every file written, every duration |
//! | `trace` | the last resort | every HTTP request, every file already conformant |
//!
//! The line to hold is `info`'s: the sequence of `info` lines from one run
//! must **tell what the program did**, without unnecessary detail and
//! without a gap. It's what gets reread first when something went wrong,
//! and it's what Sentry keeps.
//!
//! Operations that take time or that can fail are **spans**, not events: a
//! span carries its duration and its fields, and attaches everything that
//! happens during it. A slow install then reads directly, without having
//! to subtract timestamps.
//!
//! ## The log and the display are not the same thing
//!
//! Commands write a formatted report to standard output, meant for a human
//! waiting at their terminal. That's not a log: it carries no level, no
//! field, no timestamp, and a graphical interface would show it
//! differently. The two therefore coexist on purpose — the same milestone
//! appears once as text for the user, once as a structured event for
//! diagnosis.
//!
//! ## What never leaves
//!
//! The launcher holds Microsoft, Xbox Live and Minecraft tokens. Sentry's
//! documentation suggests `send_default_pii: true`; here it's the
//! opposite, and every outgoing piece of text goes through [`redact`]
//! before it's sent. A slightly less precise incident report costs
//! infinitely less than a Microsoft account token published on a
//! dashboard.
//!
//! Reporting is disabled with `SAMFLIX_TELEMETRY=0`, and the DSN is
//! overridden with `SENTRY_DSN`. The deployment environment is declared
//! with `SAMFLIX_ENV` and defaults to `local` — see [`environment`].

mod console;
pub mod environment;
mod file;
#[cfg(test)]
mod fixtures;
mod guard;
pub mod incidents;
mod init;
pub mod redact;

pub use environment::Environment;
pub use guard::{Guard, log_dir};
pub use incidents::{capture_game_crash, flush_incidents, send_test_event, telemetry_active};
pub use init::init;
pub use redact::redact;

/// A boxed logging layer.
///
/// Their number varies — no file layer if the directory is read-only, no
/// Sentry if telemetry is off — and tracing-subscriber's nested types
/// don't accommodate that.
pub(crate) type BoxedLayer =
    Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>;
