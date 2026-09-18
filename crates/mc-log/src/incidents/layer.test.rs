use super::layer;

/// The layer assembles, and accepts all five levels without panicking.
///
/// The event filter is a closure: it's only called as an event passes.
/// Without a subscriber set, it would never run at all — and it's
/// exactly the one that decides what opens an incident.
///
/// No Sentry client is initialized here: `sentry_tracing` classifies the
/// events, but nothing goes out over the network.
#[test]
fn every_level_passes_through_the_layer_without_incident() {
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry().with(layer());
    tracing::subscriber::with_default(subscriber, || {
        // `error` opens an incident, `warn` and `info` leave a
        // breadcrumb, `debug` too, `trace` is ignored.
        tracing::error!("an error");
        tracing::warn!("a warning");
        tracing::info!("a milestone");
        tracing::debug!("a detail");
        tracing::trace!("the last resort");
    });
}

/// Returns the Sentry events produced by what the closure logs.
fn incidents_from(work: impl FnOnce()) -> Vec<sentry::protocol::Event<'static>> {
    use tracing_subscriber::layer::SubscriberExt;

    sentry::test::with_captured_events(|| {
        let subscriber = tracing_subscriber::registry().with(layer());
        tracing::subscriber::with_default(subscriber, work);
    })
}

/// `error!` is the only entry point for an incident: it's how what
/// breaks on a player's machine reaches us. A filter that stops opening
/// it breaks nothing visible — it only makes the dashboard go silent.
#[test]
fn an_error_opens_an_incident() {
    let incidents = incidents_from(|| tracing::error!("the pack won't install"));

    assert_eq!(incidents.len(), 1, "{incidents:?}");
    let message = incidents[0].message.clone().unwrap_or_default();
    assert!(message.contains("the pack won't install"), "{message}");
}

/// A warning doesn't open an incident: it leaves a breadcrumb, which the
/// next incident carries along with it. That's what lets you read what
/// the launcher was doing right before it went down — without it, only
/// the bare error remains, with no context.
#[test]
fn a_warning_leaves_a_breadcrumb_in_the_next_incident() {
    let incidents = incidents_from(|| {
        tracing::warn!("retrying the download");
        tracing::info!("installing the pack");
        tracing::error!("out of disk space");
    });

    assert_eq!(incidents.len(), 1, "a single incident: the error");
    let crumbs: Vec<String> = incidents[0]
        .breadcrumbs
        .iter()
        .map(|crumb| crumb.message.clone().unwrap_or_default())
        .collect();
    assert!(
        crumbs.iter().any(|m| m.contains("retrying the download")),
        "{crumbs:?}"
    );
    assert!(
        crumbs.iter().any(|m| m.contains("installing the pack")),
        "{crumbs:?}"
    );
}
