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

/// Adoptium renvoie parfois autre chose que ce qu'on a demandé. L'entrée est
/// alors écartée, et la recherche continue sur l'image suivante : la retenir
/// installerait un paquet dont on ne sait pas ce qu'il contient, sous un nom
/// qui prétend le contraire.
#[cfg(unix)]
#[tokio::test]
async fn une_image_d_un_autre_type_que_celui_demande_est_ecartee() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install-mauvais-type");

    // Une seule réponse, deux entrées. La première ne porte pas le type
    // demandé et livrerait un Java 17 ; c'est la seconde qu'il faut retenir.
    // (Le serveur d'essai ignore la chaîne de requête : les deux images
    // partagent le même chemin, ce qui est précisément la situation où seul
    // `image_type` permet de les distinguer.)
    let intrus = archive_temurin("17.0.9");
    serveur.octets("/intrus.tar.gz", &intrus);
    let attendu = archive_temurin("21.0.5+11");
    serveur.octets("/temurin.tar.gz", &attendu);

    let deux_entrees = format!(
        "[{},{}]",
        une_entree(
            "jdk",
            &serveur.url("/intrus.tar.gz"),
            "intrus.tar.gz",
            &sha256(&intrus)
        ),
        une_entree(
            "jre",
            &serveur.url("/temurin.tar.gz"),
            "temurin.tar.gz",
            &sha256(&attendu)
        ),
    );
    serveur.json(&chemin(21, "jre"), &deux_entrees);

    let java = install_depuis(&serveur.base(), 21, &arbre.racine)
        .await
        .expect("l'entrée du bon type est retenue");
    assert_eq!(java.version.major, 21);
}

/// Une entrée de la réponse Adoptium, sans les crochets : pour en composer
/// plusieurs dans une même réponse.
fn une_entree(image: &str, lien: &str, nom: &str, sha256: &str) -> String {
    let seule = reponse_adoptium(image, lien, nom, sha256);
    seule
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

/// Le dernier contrôle, et le seul qui prouve quelque chose : le binaire
/// installé démarre et annonce la version exigée. Adoptium peut publier sous
/// un nom ce qu'il livre sous un autre, et un Java trop vieux arrête le jeu sur
/// `UnsupportedClassVersionError` avant même d'afficher une fenêtre.
#[cfg(unix)]
#[tokio::test]
async fn un_temurin_qui_annonce_une_version_trop_basse_est_refuse() {
    let serveur = mc_essais::Serveur::neuf().await;
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("install-trop-vieux");

    let archive = archive_temurin("17.0.9");
    serveur.octets("/temurin.tar.gz", &archive);
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

    let erreur = install_depuis(&serveur.base(), 21, &arbre.racine)
        .await
        .expect_err("un Java 17 ne répond pas d'une demande de Java 21");
    let texte = format!("{erreur:#}");
    assert!(texte.contains("17"), "{texte}");
    assert!(texte.contains("21"), "{texte}");
}
