use super::*;

#[test]
fn identifiant_de_version_produit() {
    assert_eq!(version_id("21.1.250"), "neoforge-21.1.250");
}

#[test]
fn url_de_l_installateur() {
    assert_eq!(
        installer_url("21.1.250"),
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.250/neoforge-21.1.250-installer.jar"
    );
}
