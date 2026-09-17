use super::{scrub_event, scrub_log_attribute, scrub_value};

#[test]
fn un_attribut_de_journal_structure_est_censure() {
    use sentry::protocol::{LogAttribute, Value};

    // Les journaux structurés passent par `before_send_log`, pas par
    // `before_send` : ce filtre-là est le seul à les voir.
    let mut attribut = LogAttribute(Value::String(
        "access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
    ));
    scrub_log_attribute(&mut attribut);
    match &attribut.0 {
        Value::String(text) => {
            assert!(!text.contains("eyJhbGci"));
            assert!(text.contains("[secret]"));
        }
        other => panic!("type inattendu : {other:?}"),
    }
}

#[test]
fn un_attribut_numerique_reste_exploitable() {
    use sentry::protocol::{LogAttribute, Value};

    // Tailles, durées, codes HTTP : rien à censurer, et ils servent au tri.
    let mut attribut = LogAttribute(Value::from(2155935));
    scrub_log_attribute(&mut attribut);
    assert_eq!(attribut.0, Value::from(2155935));
}

#[test]
fn la_pile_jointe_a_un_evenement_sans_exception_est_censuree() {
    use sentry::protocol::{Event, Frame, Stacktrace, Thread};

    // `attach_stacktrace` joint la pile du fil courant, hors de toute
    // exception : c'est le seul endroit où elle arrive pour un
    // `capture_message` ou un plantage du jeu.
    let mut event = Event::default();
    event.threads.values.push(Thread {
        stacktrace: Some(Stacktrace {
            frames: vec![Frame {
                abs_path: Some(
                    "appel avec token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
                ),
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    });

    scrub_event(&mut event);

    let chemin = event.threads.values[0].stacktrace.as_ref().unwrap().frames[0]
        .abs_path
        .clone()
        .unwrap();
    assert!(!chemin.contains("eyJhbGci"), "jeton en clair : {chemin}");
    assert!(chemin.contains("[secret]"));
}

#[test]
fn les_champs_d_un_evenement_sont_censures() {
    use sentry::protocol::{Context, Event, Value};

    // `sentry-tracing` ne remplit pas `extra` : les champs d'un
    // `tracing::error!` atterrissent dans ce contexte-là, et lui seul.
    let mut event = Event::default();
    event.contexts.insert(
        "Rust Tracing Fields".into(),
        Context::Other(
            [(
                "erreur".to_string(),
                Value::String(
                    "GET https://api/x?token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ".into(),
                ),
            )]
            .into_iter()
            .collect(),
        ),
    );

    scrub_event(&mut event);

    let rendu = format!("{:?}", event.contexts);
    assert!(
        !rendu.contains("eyJhbGci"),
        "jeton envoyé en clair : {rendu}"
    );
    assert!(rendu.contains("[secret]"));
}

/// Un secret ne se trouve pas toujours à la racine : les variables d'une trace
/// d'appels et les contextes d'un événement sont des objets qui contiennent des
/// listes qui contiennent des objets. La censure descend donc, et ne pas
/// descendre ne casse rien de visible — cela laisse seulement passer le jeton
/// d'un cran plus bas.
#[test]
fn la_censure_descend_dans_les_listes_et_les_objets() {
    use sentry::protocol::Value;

    let jeton = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ";
    let mut valeur = Value::Array(vec![
        Value::String(format!("access_token={jeton}")),
        Value::Object(
            [(
                "entete".to_string(),
                Value::Array(vec![Value::String(format!(
                    "Authorization: Bearer {jeton}"
                ))]),
            )]
            .into_iter()
            .collect(),
        ),
        // Ce qui n'est pas du texte traverse intact : les nombres et les
        // booléens servent au tri, et n'ont rien à cacher.
        Value::from(42),
    ]);

    scrub_value(&mut valeur);

    let rendu = format!("{valeur:?}");
    assert!(!rendu.contains("eyJhbGci"), "jeton en clair : {rendu}");
    // Le premier niveau, puis celui qui se cache deux crans plus bas.
    assert!(rendu.contains("access_token=[secret]"), "{rendu}");
    assert!(rendu.contains("Authorization: [secret]"), "{rendu}");
    assert!(rendu.contains("42"), "{rendu}");
}
