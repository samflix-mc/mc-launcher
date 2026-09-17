use super::{
    maven_path, mojang_arch, mojang_os, nom_mojang_de_l_architecture, nom_mojang_du_systeme,
};

/// Ces noms ne sont pas les nôtres : ce sont les clés sous lesquelles Mojang
/// publie ses bibliothèques natives. « osx » et non « macos », « arm64 » et
/// non « aarch64 » — s'en écarter fait télécharger un fichier qui n'existe pas.
#[test]
fn chaque_systeme_porte_le_nom_que_mojang_publie() {
    assert_eq!(nom_mojang_du_systeme("macos"), "osx");
    assert_eq!(nom_mojang_du_systeme("windows"), "windows");
    assert_eq!(nom_mojang_du_systeme("linux"), "linux");
    // Un système qu'on n'attendait pas est traité comme un Unix plutôt que
    // refusé : c'est le cas des BSD.
    assert_eq!(nom_mojang_du_systeme("freebsd"), "linux");
}

#[test]
fn chaque_architecture_porte_le_nom_que_mojang_publie() {
    assert_eq!(nom_mojang_de_l_architecture("x86"), "x86");
    assert_eq!(nom_mojang_de_l_architecture("aarch64"), "arm64");
    assert_eq!(nom_mojang_de_l_architecture("x86_64"), "x86_64");
    assert_eq!(nom_mojang_de_l_architecture("riscv64"), "x86_64");
}

/// La table ne sert à rien si ce qui l'interroge ne lui donne pas le poste
/// courant : ces deux-là sont la seule chose que le reste du crate appelle.
#[test]
fn le_poste_courant_est_nomme_par_la_meme_table() {
    assert_eq!(mojang_os(), nom_mojang_du_systeme(std::env::consts::OS));
    assert_eq!(
        mojang_arch(),
        nom_mojang_de_l_architecture(std::env::consts::ARCH)
    );
}

#[test]
fn chemin_maven_avec_et_sans_classifier() {
    assert_eq!(
        maven_path("org.lwjgl:lwjgl:3.3.3").unwrap(),
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
    );
    assert_eq!(
        maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap(),
        "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar"
    );
    assert_eq!(
        maven_path("net.neoforged:neoforge:21.1.250:client").unwrap(),
        "net/neoforged/neoforge/21.1.250/neoforge-21.1.250-client.jar"
    );
    assert!(maven_path("incomplet").is_none());
}
