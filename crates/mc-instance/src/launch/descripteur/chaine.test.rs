use super::{library_key, resolve_chain};
use crate::essais::{Arbre, NEOFORGE, VANILLA};

#[test]
fn la_cle_de_bibliotheque_ignore_la_version() {
    // C'est ce qui permet de voir qu'une bibliothèque en remplace une autre.
    assert_eq!(
        library_key("com.google.guava:guava:32.1.2-jre"),
        "com.google.guava:guava"
    );
    assert_eq!(
        library_key("com.google.guava:guava:31.0-jre"),
        library_key("com.google.guava:guava:32.1.2-jre")
    );
}

#[test]
fn le_classifier_distingue_deux_bibliotheques() {
    // lwjgl et lwjgl:natives-linux sont deux fichiers, tous deux nécessaires.
    assert_ne!(
        library_key("org.lwjgl:lwjgl:3.3.3"),
        library_key("org.lwjgl:lwjgl:3.3.3:natives-linux")
    );
}

#[test]
fn un_nom_sans_deux_points_reste_lui_meme() {
    assert_eq!(library_key("bizarre"), "bizarre");
}

/// Le descripteur d'un chargeur ne contient qu'un delta et désigne son socle
/// par `inheritsFrom` : il faut remonter la chaîne pour avoir le tout.
#[test]
fn la_chaine_remonte_du_chargeur_jusqu_au_socle() {
    let arbre = Arbre::neuf("chaine");
    arbre
        .version("1.21.1", VANILLA)
        .version("neoforge-21.1.250", NEOFORGE);

    let chaine = resolve_chain(&arbre.shared(), "neoforge-21.1.250").unwrap();

    assert_eq!(chaine.len(), 2);
    assert_eq!(chaine[0].id, "neoforge-21.1.250");
    assert_eq!(chaine[1].id, "1.21.1");
    // Le chargeur ne déclare pas d'index d'assets : c'est le socle qui le porte.
    assert!(chaine[0].asset_index.is_none());
    assert!(chaine[1].asset_index.is_some());
}

#[test]
fn une_version_vanilla_seule_forme_une_chaine_d_un_maillon() {
    let arbre = Arbre::neuf("chaine-seule");
    arbre.version("1.21.1", VANILLA);

    let chaine = resolve_chain(&arbre.shared(), "1.21.1").unwrap();
    assert_eq!(chaine.len(), 1);
}

/// Le message doit nommer le fichier absent : c'est lui qu'il faut
/// réinstaller, et un joueur ne devinera pas lequel.
#[test]
fn une_version_absente_nomme_le_fichier_attendu() {
    let arbre = Arbre::neuf("chaine-absente");
    let erreur = resolve_chain(&arbre.shared(), "1.21.1").expect_err("rien n'est installé");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("1.21.1 n'est pas installé"), "{texte}");
    assert!(texte.contains("1.21.1.json"), "{texte}");
}

#[test]
fn un_descripteur_illisible_est_signale_comme_tel() {
    let arbre = Arbre::neuf("chaine-cassee");
    arbre.version("1.21.1", "{ceci n'est pas du JSON");

    let erreur = resolve_chain(&arbre.shared(), "1.21.1").expect_err("JSON invalide");
    assert!(format!("{erreur:#}").contains("illisible"), "{erreur:#}");
}

/// Une boucle d'héritage — ou une chaîne absurdement profonde — ne doit pas
/// faire tourner le launcher indéfiniment.
#[test]
fn une_chaine_qui_boucle_s_arrete() {
    let arbre = Arbre::neuf("chaine-boucle");
    arbre
        .version("a", r#"{"id":"a","inheritsFrom":"b"}"#)
        .version("b", r#"{"id":"b","inheritsFrom":"a"}"#);

    let erreur = resolve_chain(&arbre.shared(), "a").expect_err("la chaîne boucle");
    assert!(
        format!("{erreur:#}").contains("trop profonde"),
        "{erreur:#}"
    );
}
