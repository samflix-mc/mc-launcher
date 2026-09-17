use super::install_panic_hook;

/// Le gestionnaire standard écrit le message de panique directement sur la
/// sortie d'erreur, sans passer par `tracing` : il échappe donc à la censure.
/// Un jeton présent dans un message de panique s'affichait ainsi en clair —
/// constaté avant d'écrire ceci.
#[test]
fn une_panique_ne_recrache_pas_un_jeton_en_clair() {
    use std::sync::{Arc, Mutex};

    let vu: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Le message part aussi en `warn`, pour atterrir dans le fichier de
    // journal : la panique y restait absente, alors que c'est le fichier qu'on
    // demande de joindre.
    struct Espion(Arc<Mutex<Vec<String>>>);
    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Espion {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct Visiteur<'a>(&'a mut Vec<String>);
            impl tracing::field::Visit for Visiteur<'_> {
                fn record_debug(
                    &mut self,
                    _field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    self.0.push(format!("{value:?}"));
                }
            }
            event.record(&mut Visiteur(&mut self.0.lock().unwrap()));
        }
    }

    let precedent = std::panic::take_hook();
    install_panic_hook();

    {
        use tracing_subscriber::layer::SubscriberExt;
        let souscripteur = tracing_subscriber::registry().with(Espion(vu.clone()));
        tracing::subscriber::with_default(souscripteur, || {
            let _ = std::panic::catch_unwind(|| {
                panic!("échec avec access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ");
            });
        });
    }

    std::panic::set_hook(precedent);

    let champs = vu.lock().unwrap().join(" | ");
    assert!(
        champs.contains("panique"),
        "la panique n'est pas arrivée dans le journal : {champs}"
    );
    assert!(
        !champs.contains("eyJhbGci"),
        "jeton en clair dans le journal : {champs}"
    );
}
