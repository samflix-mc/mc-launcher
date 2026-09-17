use super::install_depuis;
use crate::adoptium::platform;
use crate::essais::{Arbre, archive_temurin, reponse_adoptium, sha256};
use crate::version::Origin;

/// Le chemin de l'API tel que `url_assets` le compose, pour poser la réponse
/// du serveur au bon endroit.
fn chemin(major: u32, image: &str) -> String {
    let (_os, _arch) = platform().unwrap();
    format!("/assets/latest/{major}/hotspot?image_type={image}")
        .split('?')
        .next()
        .unwrap()
        .to_string()
}

#[cfg(unix)]
#[tokio::test]
async fn un_temurin_est_telecharge_depaquete_et_verifie() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install");
    let archive = archive_temurin("21.0.5+11");

    serveur.octets("/temurin.tar.gz", &archive);
    serveur.json(
        &chemin(21, "jre"),
        &reponse_adoptium(
            "jre",
            &serveur.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&archive),
        ),
    );

    let java = install_depuis(&serveur.base(), 21, &arbre.racine)
        .await
        .expect("l'installation aboutit");

    assert_eq!(java.origin, Origin::Managed);
    assert_eq!(java.version.major, 21);
    assert_eq!(
        java.path,
        arbre.racine.join("temurin-21").join("bin").join("java")
    );
    // L'archive et le répertoire d'extraction ne doivent rien laisser derrière.
    assert!(!arbre.racine.join("temurin.tar.gz").exists());
    assert!(!arbre.racine.join(".temurin-21-extraction").exists());
}

/// Adoptium ne publie pas de JRE pour toutes les combinaisons de plateformes,
/// d'où le repli sur le JDK — deux fois plus lourd, mais présent partout.
#[cfg(unix)]
#[tokio::test]
async fn a_defaut_de_jre_le_jdk_est_pris() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install-jdk");
    let archive = archive_temurin("21.0.5+11");

    serveur.octets("/temurin.tar.gz", &archive);
    // Le JRE existe dans la réponse mais sous un autre type d'image : c'est le
    // cas réel où Adoptium renvoie autre chose que ce qu'on a demandé.
    serveur.json(&chemin(21, "jre"), "[]");
    serveur.json(
        &chemin(21, "jdk"),
        &reponse_adoptium(
            "jdk",
            &serveur.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&archive),
        ),
    );

    let java = install_depuis(&serveur.base(), 21, &arbre.racine)
        .await
        .expect("le JDK prend le relais");
    assert_eq!(java.version.major, 21);
}

/// Un JDK est du code exécuté avec les droits de l'utilisateur : une archive
/// dont l'empreinte ne correspond pas ne doit jamais être dépaquetée.
#[cfg(unix)]
#[tokio::test]
async fn une_archive_dont_l_empreinte_est_fausse_n_est_pas_installee() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install-empreinte");

    serveur.octets("/temurin.tar.gz", b"<html>page d'erreur</html>");
    serveur.json(
        &chemin(21, "jre"),
        &reponse_adoptium(
            "jre",
            &serveur.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&archive_temurin("21.0.5+11")),
        ),
    );

    let erreur = install_depuis(&serveur.base(), 21, &arbre.racine)
        .await
        .expect_err("l'empreinte ne correspond pas");

    assert!(format!("{erreur:#}").contains("SHA-256"), "{erreur:#}");
    assert!(!arbre.racine.join("temurin-21").exists());
}

#[tokio::test]
async fn un_java_que_personne_ne_publie_se_dit_clairement() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install-absent");
    serveur.json(&chemin(99, "jre"), "[]");
    serveur.json(&chemin(99, "jdk"), "[]");

    let erreur = install_depuis(&serveur.base(), 99, &arbre.racine)
        .await
        .expect_err("rien de publié");

    assert!(
        format!("{erreur:#}").contains("ne publie pas de Java 99"),
        "{erreur:#}"
    );
}

#[tokio::test]
async fn une_reponse_illisible_nomme_le_type_d_image_demande() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install-illisible");
    serveur.json(&chemin(21, "jre"), "ceci n'est pas du JSON");

    let erreur = install_depuis(&serveur.base(), 21, &arbre.racine)
        .await
        .expect_err("réponse cassée");

    assert!(format!("{erreur:#}").contains("jre 21"), "{erreur:#}");
}
