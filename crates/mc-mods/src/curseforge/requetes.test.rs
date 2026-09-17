use super::super::CurseForge;
use crate::curseforge::is_key_error;
use std::sync::Arc;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn client(serveur: &mc_essais::Serveur) -> CurseForge {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForge::avec_base(dl, "$2a$10$cle-de-test".into(), &serveur.base())
}

/// Un projet et un de ses fichiers, tels que la Core API les rend.
fn projet(id: u32, slug: &str) -> String {
    format!(
        r#"{{"id":{id},"name":"{slug}","slug":"{slug}",
             "links":{{"websiteUrl":"https://www.curseforge.com/minecraft/mc-mods/{slug}"}},
             "allowModDistribution":true}}"#
    )
}

fn fichier(id: u32, mod_id: u32, nom: &str) -> String {
    format!(
        r#"{{"id":{id},"modId":{mod_id},"displayName":"{nom}","fileName":"{nom}.jar",
             "releaseType":1,"fileDate":"2026-01-01T00:00:00Z",
             "downloadUrl":"https://edge.forgecdn.net/{nom}.jar","fileLength":4096,
             "hashes":[{{"value":"abc123","algo":1}},{{"value":"def456","algo":2}}],
             "dependencies":[{{"modId":77,"relationType":3}},
                             {{"modId":88,"relationType":2}}]}}"#
    )
}

/// La clé est nominative et journalisée avec chaque requête : l'oublier ferait
/// répondre 403 à tout, et le message d'erreur ne dirait pas pourquoi.
#[tokio::test]
async fn la_cle_accompagne_chaque_requete() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/mods/search",
        &format!(r#"{{"data":[{}]}}"#, projet(1, "jei")),
    );
    serveur.json("/mods/1/files", r#"{"data":[]}"#);

    client(&serveur)
        .candidates("jei", MC, LOADER)
        .await
        .unwrap();

    let recue = &serveur.recues()[0];
    assert_eq!(recue.entete("x-api-key"), Some("$2a$10$cle-de-test"));
}

#[tokio::test]
async fn un_projet_et_ses_fichiers_deviennent_des_candidats() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/mods/search",
        &format!(r#"{{"data":[{}]}}"#, projet(1, "jei")),
    );
    serveur.json(
        "/mods/1/files",
        &format!(r#"{{"data":[{}]}}"#, fichier(5001, 1, "jei-19")),
    );

    let trouves = client(&serveur)
        .candidates("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    let candidat = &trouves[0];
    // Le SHA-1 (algo 1) est préféré au MD5 (algo 2).
    assert_eq!(candidat.sha1.as_deref(), Some("abc123"));
    assert!(candidat.sha512.is_none());
    // Seules les dépendances obligatoires (relationType 3) comptent.
    assert_eq!(candidat.declared_deps.len(), 1);
    assert_eq!(candidat.declared_deps[0].project_id, "77");
    assert!(candidat.redistributable);
}

/// Un identifiant numérique désigne directement le projet ; un slug passe par
/// la recherche. Confondre les deux ferait une requête vouée à l'échec pour
/// chaque dépendance résolue.
#[tokio::test]
async fn un_identifiant_numerique_evite_la_recherche() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/mods/238222",
        &format!(r#"{{"data":{}}}"#, projet(238222, "jei")),
    );
    serveur.json("/mods/238222/files", r#"{"data":[]}"#);

    client(&serveur)
        .candidates("238222", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(serveur.appels("/mods/search"), 0);
}

/// Un auteur peut interdire le téléchargement par un tiers. Reconstruire
/// l'URL du CDN contournerait ce refus : l'absence est propagée telle quelle.
#[tokio::test]
async fn un_refus_de_redistribution_est_propage() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/mods/search",
        r#"{"data":[{"id":2,"name":"Fermé","slug":"ferme",
                     "links":{"websiteUrl":"https://exemple.invalid/ferme"},
                     "allowModDistribution":false}]}"#,
    );
    serveur.json(
        "/mods/2/files",
        r#"{"data":[{"id":1,"modId":2,"displayName":"1.0","fileName":"ferme.jar",
                     "releaseType":1,"fileDate":"2026-01-01T00:00:00Z","downloadUrl":null,
                     "fileLength":1,"hashes":[],"dependencies":[]}]}"#,
    );

    let trouves = client(&serveur)
        .candidates("ferme", MC, LOADER)
        .await
        .unwrap();

    assert!(!trouves[0].redistributable);
    assert!(trouves[0].url.is_empty());
    // La page du mod est conservée pour que le message reste actionnable.
    assert!(trouves[0].page_url.is_some());
}

/// Une clé expire, se révoque. Le refus doit être reconnaissable pour que le
/// registre bascule sur l'accès sans clé au lieu de tout arrêter.
#[tokio::test]
async fn une_cle_refusee_est_reconnaissable() {
    for code in [401, 403] {
        let serveur = mc_essais::Serveur::neuf().await;
        serveur.code("/mods/search", code);

        let erreur = client(&serveur)
            .candidates("jei", MC, LOADER)
            .await
            .expect_err("clé refusée");

        assert!(is_key_error(&erreur), "pour HTTP {code} : {erreur:#}");
        let texte = format!("{erreur:#}");
        assert!(texte.contains("CURSEFORGE_API_KEY"), "{texte}");
        // La forme de la clé est la faute la plus courante : une clé de la
        // Core API commence par « $2a$10$ », ce n'est pas un UUID.
        assert!(texte.contains("$2a$10$"), "{texte}");
    }
}

/// Une panne de réseau n'est pas un refus de clé : le registre doit la laisser
/// remonter plutôt que de basculer en silence sur une source dégradée.
#[tokio::test]
async fn une_panne_ordinaire_n_est_pas_prise_pour_un_refus_de_cle() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/mods/search", 500);

    let erreur = client(&serveur)
        .candidates("jei", MC, LOADER)
        .await
        .expect_err("panne serveur");

    assert!(!is_key_error(&erreur), "{erreur:#}");
}

/// Un projet absent de CurseForge est un cas nominal, pas une erreur.
#[tokio::test]
async fn un_projet_inconnu_ne_donne_aucun_candidat() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/mods/search", 404);

    let trouves = client(&serveur)
        .candidates("inconnu", MC, LOADER)
        .await
        .unwrap();
    assert!(trouves.is_empty());
}

/// La route d'un fichier isolé exige aussi l'identifiant du projet ; on passe
/// donc par la recherche par identifiant de fichier, qui ne l'exige pas.
#[tokio::test]
async fn un_build_epingle_se_recupere_par_son_identifiant_de_fichier() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/mods/files",
        &format!(r#"{{"data":[{}]}}"#, fichier(5001, 1, "jei-19")),
    );
    serveur.json("/mods/1", &format!(r#"{{"data":{}}}"#, projet(1, "jei")));

    let trouve = client(&serveur)
        .candidate_by_file("5001")
        .await
        .unwrap()
        .expect("le build existe");

    assert_eq!(trouve.version_id, "5001");
    assert_eq!(trouve.slug, "jei");
    assert_eq!(serveur.recues()[0].methode, "POST");
}

#[tokio::test]
async fn un_identifiant_de_fichier_non_numerique_est_refuse() {
    let serveur = mc_essais::Serveur::neuf().await;
    let erreur = client(&serveur)
        .candidate_by_file("abcdef")
        .await
        .expect_err("pas un nombre");

    assert!(format!("{erreur:#}").contains("abcdef"), "{erreur:#}");
}

#[tokio::test]
async fn un_build_epingle_disparu_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/mods/files", r#"{"data":[]}"#);

    assert!(
        client(&serveur)
            .candidate_by_file("5001")
            .await
            .unwrap()
            .is_none()
    );
}

/// Le slug ne coïncide pas toujours avec le `modId` : `bookshelf` est publié
/// sous `bookshelf-lib`. D'où la recherche en second recours.
#[tokio::test]
async fn la_recherche_par_modid_passe_par_les_resultats_de_recherche() {
    let serveur = mc_essais::Serveur::neuf().await;
    // Premier appel : la recherche par slug ne trouve rien d'exploitable.
    serveur.json(
        "/mods/search",
        &format!(r#"{{"data":[{}]}}"#, projet(42, "bookshelf-lib")),
    );
    serveur.json(
        "/mods/42/files",
        &format!(r#"{{"data":[{}]}}"#, fichier(1, 42, "bookshelf")),
    );

    let trouves = client(&serveur)
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1);
    assert_eq!(trouves[0].slug, "bookshelf-lib");
}
