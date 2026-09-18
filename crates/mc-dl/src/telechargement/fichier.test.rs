use super::write_atomic;
use crate::{Check, Checksum, Downloader, Fetched};

fn dossier(nom: &str) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "mc-dl-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&chemin).ok();
    chemin
}

fn client() -> Downloader {
    Downloader::new(crate::USER_AGENT).unwrap()
}

fn sha1(octets: &[u8]) -> Checksum {
    let sum = Checksum::Sha1(String::new());
    Checksum::Sha1(sum.of(octets))
}

#[test]
fn ecriture_atomique_sans_reliquat() {
    let dir = dossier("atomique");
    let dest = dir.join("sous/dossier/fichier.jar");
    write_atomic(&dest, b"contenu").unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"contenu");
    // Le `.part` ne doit pas survivre au renommage.
    assert!(!dest.with_extension("jar.part").exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn un_fichier_sans_extension_recoit_quand_meme_un_part() {
    let dir = dossier("atomique-sans-ext");
    let dest = dir.join("LICENSE");
    write_atomic(&dest, b"texte").unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"texte");
    assert!(!dir.join("LICENSE.part").exists());
    std::fs::remove_dir_all(&dir).ok();
}

/// Un CDN qui renvoie une page d'erreur en HTTP 200 est détecté ici, pas trois
/// heures plus tard sous la forme d'un crash de NeoForge.
#[tokio::test]
async fn un_contenu_qui_ne_correspond_pas_a_son_empreinte_n_est_pas_ecrit() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/jei.jar", b"<html>404 not found</html>");
    let dir = dossier("empreinte-fausse");
    let dest = dir.join("jei.jar");

    let attendu = sha1(b"le vrai jar");
    let erreur = client()
        .to_file(&serveur.url("/jei.jar"), &dest, Check::Full(&attendu))
        .await
        .expect_err("l'empreinte ne correspond pas");

    assert!(format!("{erreur:#}").contains("SHA-1"), "{erreur:#}");
    assert!(!dest.exists(), "un fichier faux a été posé sur le disque");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn un_contenu_conforme_est_ecrit_et_annonce_comme_telecharge() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/jei.jar", b"le vrai jar");
    let dir = dossier("empreinte-bonne");
    let dest = dir.join("jei.jar");

    let venu = client()
        .to_file(
            &serveur.url("/jei.jar"),
            &dest,
            Check::Full(&sha1(b"le vrai jar")),
        )
        .await
        .unwrap();

    assert_eq!(venu, Fetched::Downloaded);
    assert_eq!(std::fs::read(&dest).unwrap(), b"le vrai jar");
    std::fs::remove_dir_all(&dir).ok();
}

/// Relancer une installation interrompue reprend où elle en était, ce qui
/// compte quand il reste 2 500 objets d'assets.
#[tokio::test]
async fn un_fichier_deja_conforme_n_est_pas_retelecharge() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/jei.jar", b"le vrai jar");
    let dir = dossier("deja-la");
    let dest = dir.join("jei.jar");
    write_atomic(&dest, b"le vrai jar").unwrap();

    let venu = client()
        .to_file(
            &serveur.url("/jei.jar"),
            &dest,
            Check::Full(&sha1(b"le vrai jar")),
        )
        .await
        .unwrap();

    assert_eq!(venu, Fetched::AlreadyPresent);
    assert_eq!(serveur.appels("/jei.jar"), 0, "le réseau a été sollicité");
    std::fs::remove_dir_all(&dir).ok();
}

/// Un fichier présent mais altéré doit être repris : c'est toute la valeur de
/// recalculer l'empreinte de ce qui est exécuté.
#[tokio::test]
async fn un_fichier_altere_est_repris() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/jei.jar", b"le vrai jar");
    let dir = dossier("altere");
    let dest = dir.join("jei.jar");
    write_atomic(&dest, b"autre chose").unwrap();

    let venu = client()
        .to_file(
            &serveur.url("/jei.jar"),
            &dest,
            Check::Full(&sha1(b"le vrai jar")),
        )
        .await
        .unwrap();

    assert_eq!(venu, Fetched::Downloaded);
    assert_eq!(std::fs::read(&dest).unwrap(), b"le vrai jar");
    std::fs::remove_dir_all(&dir).ok();
}

/// Recalculer le SHA-1 de 800 Mo d'assets à chaque lancement relirait tout le
/// disque pour ne presque jamais rien trouver : la taille suffit comme
/// première barrière.
#[tokio::test]
async fn la_verification_rapide_se_contente_de_la_taille() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/objet", b"12345");
    let dir = dossier("rapide");
    let dest = dir.join("objet");
    // Même taille, contenu différent : la vérification rapide l'accepte.
    write_atomic(&dest, b"abcde").unwrap();

    let sum = sha1(b"12345");
    let venu = client()
        .to_file(
            &serveur.url("/objet"),
            &dest,
            Check::Quick { sum: &sum, size: 5 },
        )
        .await
        .unwrap();

    assert_eq!(venu, Fetched::AlreadyPresent);
    std::fs::remove_dir_all(&dir).ok();
}

/// Faute d'empreinte, la taille est le seul contrôle possible. Il suffit à
/// écarter une page d'erreur servie en HTTP 200, cas de loin le plus fréquent.
#[tokio::test]
async fn sans_empreinte_la_taille_annoncee_fait_office_de_controle() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/mod.jar", b"<html>oups</html>");
    let dir = dossier("taille");
    let dest = dir.join("mod.jar");

    let erreur = client()
        .to_file(&serveur.url("/mod.jar"), &dest, Check::Size(4096))
        .await
        .expect_err("la taille ne correspond pas");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("4096 annoncés"), "{texte}");
    assert!(!dest.exists());
    std::fs::remove_dir_all(&dir).ok();
}

/// Quand la source ne publie ni empreinte ni taille, la présence est tout ce
/// qui peut être constaté — et cela vaut mieux que de retélécharger sans fin.
#[tokio::test]
async fn sans_rien_de_publie_la_presence_suffit() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/inconnu.jar", b"contenu");
    let dir = dossier("presence");
    let dest = dir.join("inconnu.jar");

    let premier = client()
        .to_file(&serveur.url("/inconnu.jar"), &dest, Check::Presence)
        .await
        .unwrap();
    let second = client()
        .to_file(&serveur.url("/inconnu.jar"), &dest, Check::Presence)
        .await
        .unwrap();

    assert_eq!(premier, Fetched::Downloaded);
    assert_eq!(second, Fetched::AlreadyPresent);
    assert_eq!(serveur.appels("/inconnu.jar"), 1);
    std::fs::remove_dir_all(&dir).ok();
}

/// **Ce que `lire_hors_du_fil` rend, et pas seulement qu'elle n'échoue pas.**
///
/// Trois mutants y survivaient : rendre un vecteur vide, `[0]`, `[1]`. Aucun
/// test ne regardait le CONTENU, et les trois se lisaient très bien à
/// l'exécution — `mc-nouvelles` s'en sert pour relire la copie du fil hors
/// ligne, et un fil vide ou d'un octet se traite exactement comme un fil
/// illisible : la page des nouvelles serait muette, sans une erreur.
///
/// Le contenu porte des accents et un octet nul : les premiers parce que le
/// launcher lit du JSON en français, le second parce qu'une lecture qui
/// passerait par une chaîne s'arrêterait là.
#[tokio::test]
async fn une_lecture_hors_du_fil_rend_les_octets_du_fichier() {
    let dir = dossier("lecture-hors-fil");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("fil.json");
    let attendu: Vec<u8> = b"{\"billets\":[]} \xc3\xa9pingl\xc3\xa9e\x00fin".to_vec();
    std::fs::write(&source, &attendu).unwrap();

    let lu = super::lire_hors_du_fil(&source).await.unwrap();

    assert_eq!(lu, attendu);
    std::fs::remove_dir_all(&dir).ok();
}

/// Un fichier absent rend une ERREUR, et non un vecteur vide.
///
/// La distinction porte tout le hors-ligne de `mc-nouvelles` : « la copie
/// n'existe pas » demande d'aller au réseau, « la copie est vide » serait un
/// fil sans billets qu'on afficherait tel quel.
#[tokio::test]
async fn une_lecture_hors_du_fil_sur_un_absent_echoue() {
    let dir = dossier("lecture-hors-fil-absent");
    std::fs::create_dir_all(&dir).unwrap();

    let erreur = super::lire_hors_du_fil(&dir.join("nulle-part.json")).await;

    assert!(erreur.is_err(), "un fichier absent doit échouer");
    std::fs::remove_dir_all(&dir).ok();
}

/// Le pendant en écriture : ce qui a été écrit se relit à l'identique.
///
/// `ecrire_hors_du_fil` prend ses octets par valeur et les confie à une tâche
/// détachée ; rien dans les suites ne vérifiait qu'ils arrivaient entiers de
/// l'autre côté.
#[tokio::test]
async fn une_ecriture_hors_du_fil_pose_exactement_ce_qu_on_lui_donne() {
    let dir = dossier("ecriture-hors-fil");
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("copie.json");
    let octets: Vec<u8> = b"\xc3\xa9crit hors du fil\x00".to_vec();

    super::ecrire_hors_du_fil(&dest, octets.clone())
        .await
        .unwrap();

    assert_eq!(std::fs::read(&dest).unwrap(), octets);
    std::fs::remove_dir_all(&dir).ok();
}
