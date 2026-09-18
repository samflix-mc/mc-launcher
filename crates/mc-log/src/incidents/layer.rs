//! The layer that sends to Sentry, and what it sends for each level.

use tracing_subscriber::Layer;

use crate::BoxedLayer;

/// Three distinct treatments, depending on what a level means:
///
/// - `Event` opens an incident. Reserved for errors, or else the
///   dashboard fills with noise and no one looks at it anymore;
/// - `Log` feeds the structured logs, searchable and readable next to
///   the matching incident;
/// - `Breadcrumb` records what came before, and is only sent attached to
///   an incident — so it's free as long as nothing fails.
///
/// `debug` stays out of the structured logs: that's the file's level,
/// thousands of lines per install, and sending it would spend quota on a
/// detail that's read locally anyway.
pub(crate) fn layer() -> BoxedLayer {
    sentry_tracing::layer()
        .event_filter(|meta| match *meta.level() {
            tracing::Level::ERROR => {
                sentry_tracing::EventFilter::Event | sentry_tracing::EventFilter::Log
            }
            tracing::Level::WARN | tracing::Level::INFO => {
                sentry_tracing::EventFilter::Breadcrumb | sentry_tracing::EventFilter::Log
            }
            tracing::Level::DEBUG => sentry_tracing::EventFilter::Breadcrumb,
            tracing::Level::TRACE => sentry_tracing::EventFilter::Ignore,
        })
        .boxed()
}

#[cfg(test)]
#[path = "layer.test.rs"]
mod tests;
