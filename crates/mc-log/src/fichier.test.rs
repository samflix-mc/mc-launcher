use super::{RedactingWriter, file_layer_in};

/// Un répertoire de travail propre à ce test, effacé par l'appelant.
fn dossier(nom: &str) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "mc-log-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&chemin).ok();
    chemin
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
fn l_ecrivain_rend_le_nombre_d_octets_qu_on_lui_a_donnes() {
    use std::io::Write;

    // La censure raccourcit le texte. Rendre la longueur écrite ferait croire
    // à une écriture partielle, et `write_all` boucle indéfiniment dessus.
    let mut tampon = Vec::new();
    let mut writer = RedactingWriter { inner: &mut tampon };
    let ligne = b"access_token=eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJ";
    assert_eq!(writer.write(ligne).unwrap(), ligne.len());
    writer.flush().unwrap();
}

#[test]
fn la_couche_fichier_ouvre_le_journal_du_jour() {
    let dir = dossier("couche");
    let (couche, guard, rendu) = file_layer_in(dir.clone(), "mc-essai");

    assert!(couche.is_some(), "aucune couche");
    assert_eq!(rendu.as_deref(), Some(dir.as_path()));
    // Le répertoire est rendu, pas le fichier : c'est l'appender qui décide du
    // nom du jour, et il en change à minuit.
    assert!(dir.is_dir());

    drop(guard);
    drop(couche);
    std::fs::remove_dir_all(&dir).ok();
}

/// Un échec d'ouverture ne doit pas empêcher le programme de tourner : on perd
/// le journal, pas l'installation.
#[test]
fn un_repertoire_impossible_ne_coupe_pas_le_programme() {
    let bloquant = dossier("bloquant");
    // Un fichier ordinaire là où l'on attend un répertoire : `create_dir_all`
    // échoue, et c'est exactement le cas d'un disque plein ou en lecture seule.
    std::fs::write(&bloquant, b"pas un repertoire").unwrap();

    let (couche, guard, rendu) = file_layer_in(bloquant.join("logs"), "mc-essai");
    assert!(couche.is_none());
    assert!(guard.is_none());
    assert!(rendu.is_none());

    std::fs::remove_file(&bloquant).ok();
}
