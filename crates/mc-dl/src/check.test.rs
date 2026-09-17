use super::{Check, Fetched};
use crate::Checksum;

fn fichier(nom: &str, contenu: &[u8]) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "mc-dl-check-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&chemin, contenu).unwrap();
    chemin
}

fn sha1_de(octets: &[u8]) -> Checksum {
    Checksum::Sha1(Checksum::Sha1(String::new()).of(octets))
}

#[test]
fn seules_les_verifications_qui_en_ont_une_exposent_leur_empreinte() {
    let sum = sha1_de(b"x");
    assert!(Check::Full(&sum).checksum().is_some());
    assert!(Check::Quick { sum: &sum, size: 1 }.checksum().is_some());
    assert!(Check::Size(1).checksum().is_none());
    assert!(Check::Presence.checksum().is_none());
}

#[test]
fn seules_les_verifications_par_taille_exposent_une_taille() {
    let sum = sha1_de(b"x");
    assert_eq!(Check::Quick { sum: &sum, size: 7 }.size(), Some(7));
    assert_eq!(Check::Size(9).size(), Some(9));
    assert_eq!(Check::Full(&sum).size(), None);
    assert_eq!(Check::Presence.size(), None);
}

/// Pour ce qui est exécuté — jars, bibliothèques, runtimes — l'empreinte est
/// recalculée à chaque passage.
#[test]
fn la_verification_complete_relit_le_fichier() {
    let chemin = fichier("complet", b"le vrai jar");

    assert!(Check::Full(&sha1_de(b"le vrai jar")).accepts_existing(&chemin));
    assert!(!Check::Full(&sha1_de(b"autre chose")).accepts_existing(&chemin));

    std::fs::remove_file(&chemin).ok();
}

/// Un asset tronqué a la mauvaise taille ; une altération silencieuse à taille
/// constante donne au pire une texture fausse, jamais du code exécuté.
#[test]
fn la_verification_rapide_ne_regarde_que_la_taille() {
    let chemin = fichier("rapide", b"12345");
    let sum = sha1_de(b"aucun rapport");

    assert!(Check::Quick { sum: &sum, size: 5 }.accepts_existing(&chemin));
    assert!(!Check::Quick { sum: &sum, size: 6 }.accepts_existing(&chemin));
    assert!(Check::Size(5).accepts_existing(&chemin));

    std::fs::remove_file(&chemin).ok();
}

#[test]
fn la_presence_accepte_tout_ce_qui_existe() {
    let chemin = fichier("presence", b"n'importe quoi");
    assert!(Check::Presence.accepts_existing(&chemin));
    std::fs::remove_file(&chemin).ok();
}

/// Un fichier absent n'est jamais accepté, quelle que soit la sévérité — sauf
/// pour la seule présence, qui n'est consultée qu'après l'avoir constatée.
#[test]
fn un_fichier_illisible_n_est_jamais_accepte() {
    let absent = std::env::temp_dir().join("mc-dl-fichier-qui-n-existe-pas");
    let sum = sha1_de(b"x");

    assert!(!Check::Full(&sum).accepts_existing(&absent));
    assert!(!Check::Quick { sum: &sum, size: 1 }.accepts_existing(&absent));
    assert!(!Check::Size(1).accepts_existing(&absent));
}

#[test]
fn les_deux_issues_d_un_telechargement_se_distinguent() {
    // Le compte rendu d'installation les oppose : « 7 téléchargés, 2 493 déjà
    // présents » n'est pas la même information que « 2 500 téléchargés ».
    assert_ne!(Fetched::Downloaded, Fetched::AlreadyPresent);
}
