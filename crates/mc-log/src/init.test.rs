use super::install_panic_hook;

/// The standard handler writes the panic message directly to standard
/// error, without going through `tracing`: it therefore escapes redaction.
/// A token present in a panic message showed up in the clear this way —
/// observed before writing this.
#[test]
fn a_panic_does_not_leak_a_token_in_the_clear() {
    use std::sync::{Arc, Mutex};

    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // The message also goes out at `warn`, to land in the log file: the
    // panic stayed absent from it, even though it's the file players are
    // asked to attach.
    struct Spy(Arc<Mutex<Vec<String>>>);
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Spy {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct Visitor<'a>(&'a mut Vec<String>);
            impl tracing::field::Visit for Visitor<'_> {
                fn record_debug(
                    &mut self,
                    _field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    self.0.push(format!("{value:?}"));
                }
            }
            event.record(&mut Visitor(&mut self.0.lock().unwrap()));
        }
    }

    let previous = std::panic::take_hook();
    install_panic_hook();

    {
        use tracing_subscriber::layer::SubscriberExt;
        let subscriber = tracing_subscriber::registry().with(Spy(seen.clone()));
        tracing::subscriber::with_default(subscriber, || {
            let _ = std::panic::catch_unwind(|| {
                panic!("failure with access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ");
            });
        });
    }

    std::panic::set_hook(previous);

    let fields = seen.lock().unwrap().join(" | ");
    assert!(
        fields.contains("panic"),
        "the panic did not reach the log: {fields}"
    );
    assert!(
        !fields.contains("eyJhbGci"),
        "token in the clear in the log: {fields}"
    );
}
