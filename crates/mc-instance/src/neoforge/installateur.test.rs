use super::{install_client, installer_url, version_id};
use crate::essais::Arbre;

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

/// L'installateur applique des patchs binaires : sur une machine lente c'est
/// une minute pendant laquelle rien ne bouge. Le relancer quand le descripteur
/// est déjà là ferait payer cette minute à chaque lancement — et ce chemin-là
/// est aussi le seul qui fonctionne hors ligne.
#[tokio::test]
async fn une_installation_deja_faite_ne_relance_pas_l_installateur() {
    let arbre = Arbre::neuf("neoforge-deja-la");
    arbre.version("neoforge-21.1.250", r#"{"id":"neoforge-21.1.250"}"#);

    let produit = install_client(
        "21.1.250",
        &arbre.shared(),
        &arbre.racine.join("cache"),
        // Un Java qui n'existe pas : si l'installateur était relancé, l'appel
        // échouerait au lieu de rendre le chemin.
        std::path::Path::new("/usr/lib/jvm/absent/bin/java"),
        &mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap(),
    )
    .await
    .expect("le descripteur est déjà là");

    assert_eq!(
        produit,
        arbre
            .shared()
            .join("versions")
            .join("neoforge-21.1.250")
            .join("neoforge-21.1.250.json")
    );
}
