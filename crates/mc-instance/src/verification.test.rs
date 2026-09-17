use super::verify;
use crate::disposition::Layout;
use crate::essais::{Arbre, NEOFORGE, VANILLA};

/// `Layout` range `shared` sous sa racine ; l'arbre factice suit la même
/// disposition, donc sa racine fait un `Layout` valable.
fn disposition(arbre: &Arbre) -> Layout {
    Layout::new(arbre.racine.clone())
}

fn complete(nom: &str) -> Arbre {
    let arbre = Arbre::neuf(nom);
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        .bibliotheque("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar")
        .bibliotheque("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .bibliotheque("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");
    arbre
}

#[test]
fn une_installation_complete_ne_signale_rien() {
    let arbre = complete("verif-complete");
    let problemes = verify("1.21.1", "21.1.250", &disposition(&arbre), false).unwrap();
    assert!(problemes.is_empty(), "{problemes:?}");
}

#[test]
fn un_client_absent_est_signale_par_son_chemin() {
    let arbre = complete("verif-sans-client");
    std::fs::remove_file(
        arbre
            .shared()
            .join("versions")
            .join("1.21.1")
            .join("1.21.1.jar"),
    )
    .unwrap();

    let problemes = verify("1.21.1", "21.1.250", &disposition(&arbre), false).unwrap();
    assert_eq!(problemes.len(), 1, "{problemes:?}");
    assert!(problemes[0].contains("fichier manquant"), "{problemes:?}");
    assert!(problemes[0].contains("1.21.1.jar"), "{problemes:?}");
}

/// Les deux descripteurs sont contrôlés : celui de NeoForge ajoute une
/// cinquantaine de bibliothèques, et il en manque une suffit à faire échouer le
/// démarrage aussi sûrement qu'une bibliothèque vanilla.
#[test]
fn une_bibliotheque_du_chargeur_manquante_est_vue() {
    let arbre = complete("verif-lib-chargeur");
    std::fs::remove_file(
        arbre
            .shared()
            .join("libraries")
            .join("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar"),
    )
    .unwrap();

    let problemes = verify("1.21.1", "21.1.250", &disposition(&arbre), false).unwrap();
    assert_eq!(problemes.len(), 1, "{problemes:?}");
    assert!(
        problemes[0].contains("bibliothèque manquante"),
        "{problemes:?}"
    );
    assert!(problemes[0].contains("loader-4.0.24.jar"), "{problemes:?}");
}

#[test]
fn un_chargeur_non_installe_est_nomme_avec_sa_version() {
    let arbre = Arbre::neuf("verif-sans-chargeur");
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .bibliotheque("com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar");

    let problemes = verify("1.21.1", "21.1.250", &disposition(&arbre), false).unwrap();
    assert!(
        problemes.iter().any(|p| p.contains("NeoForge 21.1.250")),
        "{problemes:?}"
    );
}

/// `deep` recontrôle l'empreinte de chaque objet d'assets, ce que
/// l'installation ne fait pas pour ne pas relire 800 Mo à chaque lancement.
#[test]
fn la_verification_profonde_relit_les_assets() {
    let arbre = complete("verif-profonde");
    let empreinte = arbre.asset(b"un");
    let chemin = arbre
        .shared()
        .join("assets")
        .join("objects")
        .join(&empreinte[..2])
        .join(&empreinte);
    std::fs::write(&chemin, b"autre chose").unwrap();
    arbre.index_assets("17", std::slice::from_ref(&empreinte));

    let sans = verify("1.21.1", "21.1.250", &disposition(&arbre), false).unwrap();
    assert!(sans.is_empty(), "{sans:?}");

    let avec = verify("1.21.1", "21.1.250", &disposition(&arbre), true).unwrap();
    assert!(
        avec.iter().any(|p| p.contains("asset corrompu")),
        "{avec:?}"
    );
}

/// Quand des fichiers manquent déjà, relire 800 Mo d'assets n'apprendrait rien
/// de plus : c'est l'installation qu'il faut relancer.
#[test]
fn la_verification_profonde_ne_s_ajoute_pas_a_des_problemes_deja_trouves() {
    let arbre = complete("verif-profonde-inutile");
    std::fs::remove_file(
        arbre
            .shared()
            .join("versions")
            .join("1.21.1")
            .join("1.21.1.jar"),
    )
    .unwrap();

    let problemes = verify("1.21.1", "21.1.250", &disposition(&arbre), true).unwrap();
    assert_eq!(problemes.len(), 1, "{problemes:?}");
}

/// Sans index d'assets sur le disque, la vérification profonde n'a rien à
/// relire — et ce n'est pas une erreur, seulement une installation jeune.
#[test]
fn une_verification_profonde_sans_index_ne_signale_rien() {
    let arbre = complete("verif-profonde-vide");
    let problemes = verify("1.21.1", "21.1.250", &disposition(&arbre), true).unwrap();
    assert!(problemes.is_empty(), "{problemes:?}");
}
