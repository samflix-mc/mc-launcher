use super::*;

#[test]
fn la_telemetrie_se_coupe() {
    // SAFETY : test mono-thread, variable restaurée aussitôt.
    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "0");
    }
    assert!(!telemetry_enabled());
    assert!(dsn().is_none());

    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "1");
    }
    assert!(telemetry_enabled());

    unsafe {
        std::env::remove_var("SAMFLIX_TELEMETRY");
    }
    assert!(telemetry_enabled());
}

#[test]
fn un_dsn_vide_desactive_la_remontee() {
    unsafe {
        std::env::set_var("SENTRY_DSN", "   ");
    }
    assert!(dsn().is_none());
    unsafe {
        std::env::remove_var("SENTRY_DSN");
    }
}

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
fn le_fichier_de_journal_ne_recoit_pas_les_secrets() {
    use std::io::Write;

    // Le scénario redouté : un joueur joint son journal à un ticket. Ce qui
    // est écrit sur le disque doit déjà être censuré, pas seulement ce qui
    // part vers Sentry.
    let mut tampon = Vec::new();
    {
        let mut writer = RedactingWriter { inner: &mut tampon };
        writeln!(
            writer,
            "INFO échange abouti access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ"
        )
        .unwrap();
    }

    let ecrit = String::from_utf8(tampon).unwrap();
    assert!(
        !ecrit.contains("eyJhbGci"),
        "jeton écrit en clair : {ecrit}"
    );
    assert!(ecrit.contains("[secret]"));
    assert!(ecrit.contains("échange abouti"));
}

#[test]
fn les_journaux_vivent_sous_le_repertoire_de_donnees() {
    assert!(log_dir().ends_with("logs"));
    assert!(log_dir().starts_with(mc_dl::data_dir()));
}

#[test]
fn le_chemin_annonce_est_celui_que_l_appender_ouvre() {
    // C'est le fichier qu'on demande au joueur de joindre : le nommer sans
    // sa date l'envoyait vers un fichier absent. Le nom est calculé de notre
    // côté, donc c'est l'appender lui-même qui doit l'attester — si
    // tracing-appender change de format, ce test tombe au lieu que le
    // launcher se remette silencieusement à désigner un fichier fantôme.
    let dir = std::env::temp_dir().join(format!(
        "mc-log-appender-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let _appender = tracing_appender::rolling::daily(&dir, "mc-pack.log");
    let annonce = dir.join(current_log_name("mc-pack"));

    let existe = annonce.exists();
    std::fs::remove_dir_all(&dir).ok();
    assert!(existe, "{} n'existe pas", annonce.display());
}

#[test]
fn un_extrait_se_tronque_sans_couper_un_caractere() {
    // Un journal de jeu est plein d'accents : trancher au milieu d'un
    // caractère ferait paniquer le rapport de plantage lui-même.
    let texte = "é".repeat(200);
    let borne = truncate(&texte, 101);
    assert!(borne.ends_with('é'));
    // `strip_prefix` et non `trim_start_matches` : celui-ci ne fait rien
    // quand le marqueur manque, si bien que l'assertion passait encore le
    // jour où la troncature cessait d'en poser un.
    let extrait = borne
        .strip_prefix("[…début tronqué…]\n")
        .expect("marqueur de troncature absent");
    assert!(texte.ends_with(extrait));
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
                    "GET https://api/x?token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ"
                        .into(),
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
