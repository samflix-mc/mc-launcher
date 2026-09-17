use super::{Downloader, palier};

/// Le palier décide de ce que coûte une source qui bafouille : trop court, on
/// réessaie avant que le CDN ne se soit remis ; trop long, un modpack de mille
/// fichiers passe ses minutes à attendre. Le calcul se vérifie donc, faute de
/// quoi seule une horloge dans un test pourrait le dire.
#[test]
fn la_premiere_tentative_part_sans_attendre_et_les_suivantes_patientent() {
    assert_eq!(palier(0), std::time::Duration::ZERO);
    assert_eq!(palier(1), std::time::Duration::from_millis(400));
    assert_eq!(palier(2), std::time::Duration::from_millis(800));
}

fn client() -> Downloader {
    Downloader::new(crate::USER_AGENT).expect("le client se construit")
}

/// Modrinth demande explicitement un agent identifiable et limite plus
/// sévèrement les agents anonymes ; CurseForge le journalise avec la clé.
/// C'est ce qui rend un abus traçable jusqu'à nous.
#[tokio::test]
async fn chaque_requete_annonce_l_agent_du_launcher() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/v2/project/jei", r#"{"slug":"jei"}"#);

    client()
        .bytes(&serveur.url("/v2/project/jei"))
        .await
        .unwrap();

    let recue = &serveur.recues()[0];
    assert_eq!(recue.methode, "GET");
    assert_eq!(
        recue.entete("user-agent"),
        Some(crate::USER_AGENT),
        "agent annoncé : {:?}",
        recue.entete("user-agent")
    );
}

#[tokio::test]
async fn le_corps_de_la_reponse_est_rendu_tel_quel() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/liste", r#"{"versions":["21.1.250"]}"#);

    let octets = client().bytes(&serveur.url("/liste")).await.unwrap();
    assert_eq!(octets, br#"{"versions":["21.1.250"]}"#);
}

/// Les 5xx d'un CDN passent en quelques secondes, et un modpack fait des
/// milliers de requêtes : abandonner au premier échec rendrait une
/// installation impossible pour une panne de deux secondes.
#[tokio::test]
async fn une_panne_passagere_est_reessayee() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.echoue_puis("/instable", 2, r#"{"ok":true}"#);

    let octets = client().bytes(&serveur.url("/instable")).await.unwrap();

    assert_eq!(octets, br#"{"ok":true}"#);
    assert_eq!(serveur.appels("/instable"), 3, "le réessai n'a pas eu lieu");
}

/// Trois tentatives, pas davantage : passé ce point, l'erreur doit remonter
/// avec l'URL fautive plutôt que d'allonger l'attente.
#[tokio::test]
async fn une_panne_durable_finit_par_remonter_avec_l_url() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/mort", 500);

    let erreur = client()
        .bytes(&serveur.url("/mort"))
        .await
        .expect_err("trois échecs");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("/mort"), "{texte}");
    assert!(texte.contains("500"), "{texte}");
    assert_eq!(serveur.appels("/mort"), 3);
}

/// Le corps d'une réponse d'erreur porte souvent l'explication — « clé
/// invalide », « quota dépassé ». Le perdre oblige à deviner.
#[tokio::test]
async fn le_corps_d_une_erreur_accompagne_le_code() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code_avec("/refuse", 403, r#"{"error":"clé API invalide"}"#);

    let erreur = client()
        .bytes(&serveur.url("/refuse"))
        .await
        .expect_err("403");

    assert!(
        format!("{erreur:#}").contains("clé API invalide"),
        "{erreur:#}"
    );
}

#[tokio::test]
async fn un_chemin_inconnu_remonte_un_404() {
    let serveur = mc_essais::Serveur::neuf().await;
    let erreur = client()
        .bytes(&serveur.url("/nulle-part"))
        .await
        .expect_err("404");
    assert!(format!("{erreur:#}").contains("404"), "{erreur:#}");
}

#[test]
fn le_client_est_partageable_tel_quel() {
    // `client()` est exposé pour les appels qui ont besoin d'en-têtes propres
    // — la clé de CurseForge, le jeton Microsoft.
    let dl = client();
    assert!(std::ptr::eq(dl.client(), dl.client()));
}
