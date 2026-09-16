use super::*;

#[test]
fn serie_deduite_de_la_version_du_jeu() {
    assert_eq!(series_for("1.21.1").as_deref(), Some("21.1."));
    assert_eq!(series_for("1.21").as_deref(), Some("21.0."));
    assert_eq!(series_for("1.20.4").as_deref(), Some("20.4."));
    // NeoForge ne couvre pas les versions antérieures au versionnage 1.x.
    assert_eq!(series_for("21w07a"), None);
}

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

#[test]
fn la_queue_garde_les_dernieres_lignes() {
    assert_eq!(tail("a\nb\nc\nd", 2), "c\nd");
    assert_eq!(tail("a", 5), "a");
}
