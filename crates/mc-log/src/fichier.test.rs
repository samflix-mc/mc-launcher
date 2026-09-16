use super::*;

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
