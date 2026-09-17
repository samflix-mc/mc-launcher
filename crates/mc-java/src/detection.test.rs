use super::detect;
use crate::essais::{Arbre, MAJEUR_INTROUVABLE, faux_java, java_muet};
use crate::version::Origin;

/// Le runtime géré passe en premier : s'il est là, c'est le launcher qui l'a
/// installé et vérifié, inutile de sonder le système.
#[cfg(unix)]
#[tokio::test]
async fn le_runtime_gere_est_retenu_et_reconnu_comme_tel() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("detect-gere");
    faux_java(
        &arbre.racine.join("temurin-21").join("bin").join("java"),
        "21.0.5+11",
    );

    let java = detect(21, &arbre.racine).await.expect("il est là");

    assert_eq!(java.origin, Origin::Managed);
    assert_eq!(java.version.major, 21);
}

/// En dessous de la version exigée, le jeu s'arrête sur
/// `UnsupportedClassVersionError` avant même d'afficher une fenêtre : un
/// runtime trop vieux ne vaut pas mieux que pas de runtime.
#[cfg(unix)]
#[tokio::test]
async fn un_runtime_trop_vieux_n_est_pas_retenu() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("detect-vieux");
    faux_java(
        &arbre
            .racine
            .join(format!("temurin-{MAJEUR_INTROUVABLE}"))
            .join("bin")
            .join("java"),
        "17.0.9",
    );

    assert!(detect(MAJEUR_INTROUVABLE, &arbre.racine).await.is_none());
}

/// Un runtime plus récent que demandé convient : c'est la borne basse qui
/// compte.
#[cfg(unix)]
#[tokio::test]
async fn un_runtime_plus_recent_convient() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("detect-recent");
    faux_java(
        &arbre.racine.join("temurin-17").join("bin").join("java"),
        "21.0.5+11",
    );

    let java = detect(17, &arbre.racine).await.expect("21 ≥ 17");
    assert_eq!(java.version.major, 21);
}

/// Un exécutable peut être présent et cassé — lien symbolique mort, paquet à
/// moitié désinstallé. On ne retient que ce qui répond.
#[cfg(unix)]
#[tokio::test]
async fn un_binaire_casse_est_passe_sans_arreter_la_recherche() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("detect-casse");
    java_muet(
        &arbre
            .racine
            .join(format!("temurin-{MAJEUR_INTROUVABLE}"))
            .join("bin")
            .join("java"),
    );

    // Il ne répond rien d'exploitable : la détection continue, et ne trouve
    // rien d'autre à opposer.
    assert!(detect(MAJEUR_INTROUVABLE, &arbre.racine).await.is_none());
}

#[tokio::test]
async fn un_repertoire_vide_ne_donne_rien() {
    let _atelier = crate::essais::atelier();
    let arbre = Arbre::neuf("detect-vide");
    assert!(detect(MAJEUR_INTROUVABLE, &arbre.racine).await.is_none());
}
