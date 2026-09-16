use super::library_key;

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
