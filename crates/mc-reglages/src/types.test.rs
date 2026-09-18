use super::Fond;

/// Chaque fond nomme SON fichier, et « uni » n'en a pas.
///
/// Rien ne le vérifiait : rendre la chaîne vide pour tous les fonds — ou
/// n'importe quelle chaîne — laissait la suite verte. Le symptôme serait un
/// launcher qui affiche le même fond quel que soit le réglage, ou aucun.
#[test]
fn chaque_fond_nomme_son_fichier() {
    assert_eq!(Fond::Spawn.fichier(), "spawn.webp");
    assert_eq!(Fond::Nether.fichier(), "nether.webp");
    assert_eq!(Fond::Fin.fichier(), "fin.webp");
    // « uni » est un dégradé déclaré en CSS : pas d'image, et c'est la seule
    // valeur vide légitime.
    assert_eq!(Fond::Uni.fichier(), "");
}

/// Et deux fonds ne partagent pas de fichier.
#[test]
fn deux_fonds_ne_partagent_pas_leur_image() {
    let avec_image: Vec<&str> = Fond::TOUS
        .iter()
        .map(|fond| fond.fichier())
        .filter(|nom| !nom.is_empty())
        .collect();
    let mut uniques = avec_image.clone();
    uniques.sort_unstable();
    uniques.dedup();
    assert_eq!(uniques.len(), avec_image.len(), "{avec_image:?}");
}
