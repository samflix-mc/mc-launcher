use super::super::CurseForgeWeb;
use std::sync::Arc;

fn client(serveur: &mc_essais::Serveur) -> CurseForgeWeb {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForgeWeb::avec_bases(dl, &serveur.base(), &serveur.url("/widget"))
}

const FICHIER: &str = r#"{"id":5001,"fileName":"jei-19.jar","displayName":"JEI 19",
                          "fileLength":2048,"releaseType":2,
                          "dateCreated":"2026-01-01T00:00:00Z",
                          "gameVersions":["1.21.1","NeoForge"]}"#;

/// Un build épinglé n'est pas filtré par l'API : il est rendu tel quel, avec
/// son canal, et c'est ce qui rend une installation reproductible.
#[tokio::test]
async fn un_build_epingle_est_rendu_tel_quel() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/widget/jei",
        r#"{"id":238222,"title":"Just Enough Items"}"#,
    );
    serveur.json("/mods/238222/files/5001", FICHIER);

    let trouve = client(&serveur)
        .candidate_by_file("jei", "5001")
        .await
        .unwrap()
        .expect("le build existe");

    assert_eq!(trouve.version_id, "5001");
    assert_eq!(trouve.channel, crate::Channel::Beta);
    assert_eq!(trouve.name, "Just Enough Items");
    assert!(trouve.page_url.as_deref().unwrap().contains("jei"));
}

#[tokio::test]
async fn un_build_epingle_disparu_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/jei", r#"{"id":238222,"title":"JEI"}"#);
    serveur.code("/mods/238222/files/9999", 404);

    assert!(
        client(&serveur)
            .candidate_by_file("jei", "9999")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn un_projet_inconnu_ne_donne_pas_de_build() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/widget/inconnu", 404);

    assert!(
        client(&serveur)
            .candidate_by_file("inconnu", "1")
            .await
            .unwrap()
            .is_none()
    );
}

/// Sans clé, la recherche par mot-clé est fermée : seul un `modId` qui est
/// aussi le slug du projet peut aboutir.
#[tokio::test]
async fn la_recherche_par_modid_passe_par_le_slug() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/bookshelf", r#"{"id":42,"title":"Bookshelf"}"#);
    serveur.json(
        "/mods/42/files",
        r#"{"data":[{"id":1,"fileName":"bookshelf.jar","displayName":"Bookshelf",
                     "fileLength":1,"releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
                     "gameVersions":["1.21.1","NeoForge"]}],"pagination":{"totalCount":1}}"#,
    );
    serveur.json("/mods/42/dependencies", r#"{"data":[]}"#);

    let trouves = client(&serveur)
        .find_by_mod_id("bookshelf", "1.21.1", "neoforge")
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1);
    assert_eq!(trouves[0].slug, "bookshelf");
}
