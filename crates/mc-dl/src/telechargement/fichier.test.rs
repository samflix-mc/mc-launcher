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
