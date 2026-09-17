use super::essais::{Atelier, entree};
use super::executer;
use std::process::ExitCode;

async fn lancer(commande: &str, atelier: &Atelier) -> ExitCode {
    let source = atelier.pack_installe(vec![entree("jei", "both")]);
    executer(
        commande,
        &source,
        &atelier.options(),
        false,
        None,
        None,
        None,
        false,
    )
    .await
    .expect("la commande se termine")
}

/// `verify` est la seule à distinguer deux réussites : l'installation est
/// conforme, ou elle ne l'est pas sans que le programme ait pour autant échoué.
#[tokio::test]
async fn verify_distingue_deux_reussites() {
    let atelier = Atelier::neuf("executer-verify");
    let code = lancer("verify", &atelier).await;
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

/// Une commande inconnue rappelle l'usage et rend 2 — le code que les shells
/// attendent d'une ligne de commande mal formée.
#[tokio::test]
async fn une_commande_inconnue_rappelle_l_usage() {
    let atelier = Atelier::neuf("executer-inconnue");
    let code = lancer("instal", &atelier).await;
    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
}
