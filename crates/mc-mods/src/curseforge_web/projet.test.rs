use super::super::CurseForgeWeb;
use super::attente_du_cache;
use std::sync::Arc;

/// La seconde tentative n'a de sens que si elle laisse à cfwidget le temps de
/// constituer son cache : repartir aussitôt redemanderait un 202, et le projet
/// serait déclaré introuvable alors qu'il existe.
#[test]
fn la_seconde_tentative_laisse_au_cache_le_temps_de_se_faire() {
    assert_eq!(attente_du_cache(0), std::time::Duration::ZERO);
    assert_eq!(attente_du_cache(1), std::time::Duration::from_secs(3));
}

fn client(serveur: &mc_essais::Serveur) -> CurseForgeWeb {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForgeWeb::avec_bases(dl, &serveur.base(), &serveur.url("/widget"))
}

#[tokio::test]
async fn un_slug_donne_son_identifiant_et_son_nom() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/widget/jei",
        r#"{"id":238222,"title":"Just Enough Items"}"#,
    );

    let trouve = client(&serveur).project_id("jei").await.unwrap();
    assert_eq!(trouve, Some((238222, "Just Enough Items".to_string())));
}

/// cfwidget répond 202 le temps de constituer son cache pour un projet qu'il
/// n'a jamais vu. Abandonner au premier appel rendrait tout mod récent
/// introuvable sans clé d'API.
#[tokio::test]
async fn un_202_de_cfwidget_donne_lieu_a_une_seconde_tentative() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/widget/nouveau", 202);

    let trouve = client(&serveur).project_id("nouveau").await.unwrap();

    assert_eq!(trouve, None, "aucun résultat après deux tentatives");
    assert_eq!(serveur.appels("/widget/nouveau"), 2);
}

#[tokio::test]
async fn un_projet_inconnu_de_cfwidget_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/widget/inconnu", 404);

    assert_eq!(client(&serveur).project_id("inconnu").await.unwrap(), None);
    // Un 404 est définitif : inutile de réessayer.
    assert_eq!(serveur.appels("/widget/inconnu"), 1);
}

/// cfwidget est un service tiers bénévole : une réponse qui n'a plus la forme
/// attendue doit valoir « rien trouvé », pas arrêter l'installation.
#[tokio::test]
async fn une_reponse_illisible_vaut_absence() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/casse", r#"{"identifiant":238222}"#);

    assert_eq!(client(&serveur).project_id("casse").await.unwrap(), None);
}

/// Un identifiant numérique vient d'une dépendance déjà résolue : le nom
/// lisible n'est pas indispensable, et l'économiser évite un appel à un
/// service tiers.
#[tokio::test]
async fn un_identifiant_numerique_se_resout_sans_appel() {
    let serveur = mc_essais::Serveur::neuf().await;

    let trouve = client(&serveur).resolve_project("238222").await.unwrap();

    assert_eq!(trouve, Some((238222, "projet 238222".to_string())));
    assert!(serveur.recues().is_empty(), "{:?}", serveur.recues());
}

#[tokio::test]
async fn un_slug_passe_bien_par_cfwidget() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/jei", r#"{"id":1,"title":"JEI"}"#);

    let trouve = client(&serveur).resolve_project("jei").await.unwrap();
    assert_eq!(trouve, Some((1, "JEI".to_string())));
}
