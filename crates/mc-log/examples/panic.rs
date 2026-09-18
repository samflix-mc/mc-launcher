//! Triggers a panic to verify it becomes an incident.
//!
//!     cargo run -p mc-log --example panic
//!     SAMFLIX_TELEMETRY=0 cargo run -p mc-log --example panic   # without sending
//!
//! Complements `mc-pack diagnostic --incident-test`, which only validates
//! the transport: here it's the panic handler that's exercised, from the
//! panic to the send. The message deliberately contains a fake token, so
//! you can verify in Sentry that it comes out redacted.

fn main() {
    let _log = mc_log::init("mc-log-panic");

    tracing::info!("before the panic — this message should become a breadcrumb");
    tracing::warn!("a warning, to verify it accompanies the incident");

    panic!(
        "deliberate verification panic, with access_token=\
         eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.dGVzdA that must not leak out"
    );
}
