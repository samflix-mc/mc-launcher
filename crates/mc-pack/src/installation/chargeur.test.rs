use super::version;
use crate::essais::{Atelier, verrou};
use crate::manifest::Manifest;

fn client() -> mc_dl::Downloader {
    mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap()
}

fn manifeste(loader_version: &str) -> Manifest {
    let brut = format!(
        r#"{{"schema":1,"name":"samflix","minecraft":"1.21.1",
             "loader":{{"type":"neoforge","version":"{loader_version}"}}}}"#
    );
    Manifest::parse(brut.as_bytes()).unwrap()
}

/// Le verrou fait foi quand on le rejoue : un joueur ne choisit pas sa version
/// de NeoForge, sinon il arriverait sur le serveur avec un chargeur différent.
#[tokio::test]
async fn le_rejeu_prend_la_version_du_verrou() {
    let atelier = Atelier::neuf("chargeur-rejeu");
    let chemin = atelier.racine.join("samflix.lock.json");

    let posee = version(
        &manifeste("latest"),
        Some(&verrou(Vec::new())),
        &chemin,
        true,
        &client(),
    )
    .await
    .unwrap();

    // « latest » du manifeste est ignoré : c'est le verrou qui décide, et
    // aucun appel réseau n'a lieu.
    assert_eq!(posee, "21.1.250");
}

/// Une version épinglée au manifeste est prise telle quelle : c'est ce que
/// l'épinglage sert à obtenir.
#[tokio::test]
async fn une_version_epinglee_est_prise_telle_quelle() {
    let atelier = Atelier::neuf("chargeur-epingle");
    let chemin = atelier.racine.join("samflix.lock.json");

    let posee = version(&manifeste("21.1.100"), None, &chemin, false, &client())
        .await
        .unwrap();

    assert_eq!(posee, "21.1.100");
}

/// Rejouer sans verrou n'a pas de sens, et le message doit nommer le fichier
/// qu'on attendait.
#[tokio::test]
async fn un_rejeu_sans_verrou_nomme_le_fichier_attendu() {
    let atelier = Atelier::neuf("chargeur-sans-verrou");
    let chemin = atelier.racine.join("samflix.lock.json");

    let erreur = version(&manifeste("latest"), None, &chemin, true, &client())
        .await
        .expect_err("rien à rejouer");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("samflix.lock.json"), "{texte}");
    assert!(texte.contains("rien à rejouer"), "{texte}");
}
