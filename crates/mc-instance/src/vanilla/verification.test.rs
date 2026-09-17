use super::{classpath, verify_assets};
use crate::essais::{Arbre, NEOFORGE, VANILLA};

fn descripteur(arbre: &Arbre, id: &str) -> std::path::PathBuf {
    arbre
        .shared()
        .join("versions")
        .join(id)
        .join(format!("{id}.json"))
}

/// Le descripteur que produit NeoForge ne porte ni `assetIndex` ni
/// `downloads` : le lire avec la structure complète échouerait, alors que ses
/// bibliothèques comptent autant que celles de Mojang.
#[test]
fn le_descripteur_d_un_chargeur_se_lit_comme_celui_de_mojang() {
    let arbre = Arbre::neuf("verif-chargeur");
    arbre.version("neoforge-21.1.250", NEOFORGE);

    let libs = classpath(&descripteur(&arbre, "neoforge-21.1.250"), &arbre.shared()).unwrap();

    assert_eq!(libs.len(), 2, "{libs:?}");
    assert!(
        libs.iter().any(|p| p.ends_with("guava-33.0.0-jre.jar")),
        "{libs:?}"
    );
    // Les chemins sont relatifs au dépôt partagé.
    assert!(
        libs.iter()
            .all(|p| p.starts_with(arbre.shared().join("libraries")))
    );
}

#[test]
fn une_bibliotheque_reservee_a_un_autre_systeme_n_est_pas_exigee() {
    let arbre = Arbre::neuf("verif-systeme");
    arbre.version("1.21.1", VANILLA);

    let libs = classpath(&descripteur(&arbre, "1.21.1"), &arbre.shared()).unwrap();

    // Sous linux, la native macOS de LWJGL n'a pas à être présente.
    assert!(
        !libs.iter().any(|p| p.to_string_lossy().contains("lwjgl")),
        "{libs:?}"
    );
}

#[test]
fn un_descripteur_illisible_est_signale_avec_son_chemin() {
    let arbre = Arbre::neuf("verif-casse");
    arbre.version("1.21.1", "pas du JSON");

    let erreur = classpath(&descripteur(&arbre, "1.21.1"), &arbre.shared()).expect_err("cassé");
    assert!(format!("{erreur:#}").contains("illisible"), "{erreur:#}");
}

/// Complément de la vérification rapide, qui ne compare que les tailles : ici
/// chaque objet est relu et son empreinte recalculée.
#[test]
fn un_asset_intact_est_compte_comme_tel() {
    let arbre = Arbre::neuf("assets-intacts");
    let empreintes = vec![arbre.asset(b"un"), arbre.asset(b"deux")];
    arbre.index_assets("17", &empreintes);

    let rapport = verify_assets(&arbre.shared(), "17").unwrap();

    assert_eq!(rapport.ok, 2);
    assert!(rapport.is_clean());
}

#[test]
fn un_asset_absent_est_nomme_par_son_empreinte() {
    let arbre = Arbre::neuf("assets-absent");
    let present = arbre.asset(b"un");
    let absent = "0000000000000000000000000000000000000000".to_string();
    arbre.index_assets("17", &[present, absent.clone()]);

    let rapport = verify_assets(&arbre.shared(), "17").unwrap();

    assert_eq!(rapport.ok, 1);
    assert_eq!(rapport.missing, vec![absent]);
    assert!(!rapport.is_clean());
}

/// Un objet dont le contenu ne correspond plus à son nom : c'est exactement ce
/// que la vérification rapide laisse passer, et ce que celle-ci doit voir.
#[test]
fn un_asset_corrompu_est_distingue_d_un_asset_absent() {
    let arbre = Arbre::neuf("assets-corrompu");
    let empreinte = arbre.asset(b"un");
    let chemin = arbre
        .shared()
        .join("assets")
        .join("objects")
        .join(&empreinte[..2])
        .join(&empreinte);
    std::fs::write(&chemin, b"autre chose").unwrap();
    arbre.index_assets("17", std::slice::from_ref(&empreinte));

    let rapport = verify_assets(&arbre.shared(), "17").unwrap();

    assert_eq!(rapport.ok, 0);
    assert!(rapport.missing.is_empty());
    assert_eq!(rapport.corrupt, vec![empreinte]);
}

#[test]
fn un_index_absent_est_une_erreur_et_non_un_rapport_vide() {
    let arbre = Arbre::neuf("assets-sans-index");
    assert!(verify_assets(&arbre.shared(), "17").is_err());
}
