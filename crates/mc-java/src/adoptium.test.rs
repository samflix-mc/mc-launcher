use super::{API, platform, url_assets};

#[test]
fn l_url_demande_le_dernier_binaire_de_la_plateforme() {
    assert_eq!(
        url_assets(API, 21, "linux", "x64", "jre"),
        "https://api.adoptium.net/v3/assets/latest/21/hotspot\
         ?architecture=x64&image_type=jre&os=linux&vendor=eclipse"
    );
}

/// Le vocabulaire d'Adoptium n'est pas celui de Rust : `macos` s'y dit `mac`,
/// `x86_64` s'y dit `x64`. Se tromper donne une liste vide, donc « Adoptium ne
/// publie pas de Java 21 » sur un poste parfaitement ordinaire.
#[test]
fn la_plateforme_courante_a_un_nom_chez_adoptium() {
    let (os, arch) = platform().expect("ce poste est couvert par Temurin");
    assert!(
        ["linux", "mac", "windows"].contains(&os),
        "système inattendu : {os}"
    );
    assert!(
        ["x64", "aarch64"].contains(&arch),
        "architecture inattendue : {arch}"
    );
}
