use super::{Path, parse_major, probe};
use crate::emplacements::{candidates, managed_home};

#[test]
fn majeur_des_deux_schemas_de_version() {
    assert_eq!(parse_major("21.0.5+11"), Some(21));
    assert_eq!(parse_major("21"), Some(21));
    assert_eq!(parse_major("17.0.9"), Some(17));
    // Jusqu'à Java 8, le majeur est le deuxième nombre.
    assert_eq!(parse_major("1.8.0_412"), Some(8));
    assert_eq!(parse_major("1.7.0_80"), Some(7));
    assert_eq!(parse_major("22-ea"), Some(22));
    assert_eq!(parse_major(""), None);
}

/// Un « 1 » seul ne dit rien : le deuxième nombre manque, et deviner ferait
/// passer un runtime inconnu pour un Java 1.
#[test]
fn un_schema_tronque_ne_donne_pas_de_majeur() {
    assert_eq!(parse_major("1"), None);
    assert_eq!(parse_major("1.x"), None);
    assert_eq!(parse_major("rien du tout"), None);
}

#[test]
fn le_runtime_gere_est_le_premier_candidat() {
    let dir = Path::new("/tmp/mc-runtime");
    let list = candidates(dir, 21);
    assert_eq!(list[0], managed_home(dir, 21).join("bin").join("java"));
}

/// `-version` écrit sur stderr — choix historique de la JVM — et sur trois
/// lignes dont seule la première porte le numéro, entre guillemets. Lire la
/// sortie standard donnerait le vide.
#[cfg(unix)]
#[tokio::test]
async fn la_version_se_lit_sur_la_sortie_d_erreur() {
    let _atelier = crate::essais::atelier();
    let arbre = crate::essais::Arbre::neuf("probe");
    let exe = arbre.racine.join("bin").join("java");
    crate::essais::faux_java(&exe, "21.0.5+11");

    let version = probe(&exe).await.expect("le binaire répond");

    assert_eq!(version.major, 21);
    assert_eq!(version.full, "21.0.5+11");
}

#[cfg(unix)]
#[tokio::test]
async fn un_java_8_est_reconnu_comme_tel() {
    let _atelier = crate::essais::atelier();
    let arbre = crate::essais::Arbre::neuf("probe-8");
    let exe = arbre.racine.join("bin").join("java");
    crate::essais::faux_java(&exe, "1.8.0_412");

    let version = probe(&exe).await.unwrap();
    assert_eq!(version.major, 8, "un Java 8 est passé pour un Java 1");
}

/// Un binaire présent mais muet — paquet à moitié désinstallé — ne doit pas
/// être retenu sur la foi de son seul chemin.
#[cfg(unix)]
#[tokio::test]
async fn un_binaire_qui_ne_dit_rien_est_refuse() {
    let _atelier = crate::essais::atelier();
    let arbre = crate::essais::Arbre::neuf("probe-muet");
    let exe = arbre.racine.join("bin").join("java");
    crate::essais::java_muet(&exe);

    let erreur = probe(&exe).await.expect_err("rien d'exploitable");
    assert!(format!("{erreur:#}").contains("illisible"), "{erreur:#}");
}

#[tokio::test]
async fn un_binaire_absent_se_dit_avec_son_chemin() {
    let erreur = probe(Path::new("/usr/lib/jvm/absent/bin/java"))
        .await
        .expect_err("rien à cette place");
    assert!(format!("{erreur:#}").contains("absent"), "{erreur:#}");
}
