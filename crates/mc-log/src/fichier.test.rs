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

/// `file_layer` ne fait qu'une chose de plus que `file_layer_in` : choisir le
/// répertoire. C'est peu, et c'est tout ce qui range le journal là où le
/// diagnostic ira le chercher — rendre trois `None` priverait silencieusement
/// le launcher de son fichier, celui-là même qu'on demande à un joueur de
/// joindre.
#[test]
fn la_couche_fichier_s_installe_dans_le_repertoire_des_journaux() {
    let vars = crate::essais::variables();
    let racine = dossier("couche-defaut");
    std::fs::create_dir_all(&racine).unwrap();
    vars.poser("XDG_DATA_HOME", racine.to_str().unwrap());

    let (couche, garde, chemin) = super::file_layer("essai");

    assert!(couche.is_some(), "aucune couche fichier");
    assert!(garde.is_some(), "aucun garde d'écriture");
    let chemin = chemin.expect("le répertoire des journaux est rendu");
    assert!(
        chemin.starts_with(&racine),
        "journal hors du répertoire déclaré : {chemin:?}"
    );
    assert_eq!(chemin, crate::guard::log_dir());

    drop(garde);
    std::fs::remove_dir_all(&racine).ok();
}

/// Un `flush` qui ne descend pas jusqu'au fichier laisse la dernière ligne
/// dans un tampon — et c'est justement celle qui dit pourquoi le launcher
/// s'est arrêté.
#[test]
fn le_vidage_traverse_jusqu_a_l_ecrivain_enveloppe() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Temoin(Arc<Mutex<usize>>);

    impl Write for Temoin {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            *self.0.lock().unwrap() += 1;
            Ok(())
        }
    }

    let temoin = Temoin::default();
    let mut ecrivain = RedactingWriter {
        inner: temoin.clone(),
    };
    ecrivain.write_all(b"une ligne\n").unwrap();
    assert_eq!(*temoin.0.lock().unwrap(), 0, "rien n'a encore été demandé");

    ecrivain.flush().unwrap();
    assert_eq!(*temoin.0.lock().unwrap(), 1, "le vidage n'est pas descendu");
}
