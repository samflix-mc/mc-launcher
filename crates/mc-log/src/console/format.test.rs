use super::ConsoleFormat;
use std::sync::{Arc, Mutex};

/// Un écrivain partagé, pour relire ce que la console aurait affiché.
#[derive(Clone)]
struct Tampon(Arc<Mutex<Vec<u8>>>);

impl Tampon {
    fn neuf() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn lu(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl std::io::Write for Tampon {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl tracing_subscriber::fmt::MakeWriter<'_> for Tampon {
    type Writer = Tampon;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

/// Émet les événements de `corps` dans une console mise en forme par
/// [`ConsoleFormat`], et rend ce qui en est sorti.
fn console(corps: impl FnOnce()) -> String {
    let tampon = Tampon::neuf();
    let souscripteur = tracing_subscriber::fmt()
        .event_format(ConsoleFormat::new())
        .with_writer(tampon.clone())
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::with_default(souscripteur, corps);
    tampon.lu()
}

/// À partir d'`info`, le message se suffit à lui-même. Répéter les champs
/// qu'il contient déjà doublerait la ligne sans rien apprendre — ils restent
/// dans le fichier et dans Sentry, où ils servent à filtrer.
#[test]
fn a_partir_d_info_seul_le_message_s_affiche() {
    let sorti = console(|| {
        tracing::info!(mods = 7, duree_ms = 1234, "instance installée");
    });

    assert!(
        sorti.contains("instance installée"),
        "message absent : {sorti}"
    );
    assert!(
        !sorti.contains("duree_ms"),
        "les champs noient la ligne : {sorti}"
    );
}

/// En `debug`, les champs *sont* l'information : le message n'est qu'une
/// étiquette au-dessus d'eux.
#[test]
fn en_debug_les_champs_sont_l_information() {
    let sorti = console(|| {
        tracing::debug!(slug = "jei", version = "19.51.0", "mod retenu");
    });

    assert!(sorti.contains("jei"), "champs absents : {sorti}");
    assert!(sorti.contains("19.51.0"), "champs absents : {sorti}");
}

/// Le temps écoulé depuis le démarrage, pas l'heure absolue : savoir qu'une
/// étape a pris 4,2 s renseigne, savoir qu'il était 01:18:38 non.
#[test]
fn chaque_ligne_porte_le_temps_ecoule_et_son_niveau() {
    let sorti = console(|| {
        tracing::warn!("clé refusée");
    });

    let ligne = sorti.lines().next().expect("une ligne au moins");
    assert!(ligne.contains('s'), "pas de durée : {ligne}");
    assert!(ligne.contains("WARN"), "pas de niveau : {ligne}");
    // Deux décimales suffisent à situer une étape ; la nanoseconde du format
    // par défaut ne sert qu'à allonger la ligne.
    assert!(
        ligne.split('s').next().unwrap().contains('.'),
        "durée sans décimale : {ligne}"
    );
}

/// L'horloge est capturée une fois à la construction. La recréer à chaque
/// ligne afficherait zéro partout — ce fut le premier essai.
#[test]
fn l_horloge_ne_repart_pas_de_zero_a_chaque_ligne() {
    let format = ConsoleFormat::new();
    std::thread::sleep(std::time::Duration::from_millis(20));

    let tampon = Tampon::neuf();
    let souscripteur = tracing_subscriber::fmt()
        .event_format(format)
        .with_writer(tampon.clone())
        .finish();
    tracing::subscriber::with_default(souscripteur, || tracing::info!("plus tard"));

    let sorti = tampon.lu();
    assert!(
        !sorti.trim_start().starts_with("0.00s"),
        "l'horloge est repartie de zéro : {sorti}"
    );
}
