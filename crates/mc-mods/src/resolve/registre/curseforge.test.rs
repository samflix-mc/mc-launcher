use crate::essais::{Atelier, Projet, Version, jar, publier};
use crate::resolve::registre::Registry;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn registre(atelier: &Atelier, serveur: &mc_essais::Serveur, cle: Option<&str>) -> Registry {
    Registry::pour_essais(atelier.racine.join("cache"), &serveur.base(), cle).unwrap()
}

/// Le registre sait s'il a une clé, et la résolution s'en sert pour décider
/// quoi annoncer au joueur quand un mod reste introuvable : sans clé, il lui
/// manque une source entière, et le lui dire évite de chercher une panne là où
/// il n'y a qu'une configuration absente.
#[tokio::test]
async fn le_registre_sait_s_il_a_une_cle() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-cle-presente");

    assert!(registre(&atelier, &serveur, Some("$2a$10$cle")).has_curseforge());
    assert!(!registre(&atelier, &serveur, None).has_curseforge());
}

/// Un projet servi par la Core API, avec sa clé.
fn publier_core(serveur: &mc_essais::Serveur, id: u32, slug: &str) {
    serveur.json(
        "/mods/search",
        &format!(
            r#"{{"data":[{{"id":{id},"name":"{slug}","slug":"{slug}",
                 "links":{{"websiteUrl":"https://exemple.invalid/{slug}"}},
                 "allowModDistribution":true}}]}}"#
        ),
    );
    serveur.json(
        &format!("/mods/{id}/files"),
        &format!(
            r#"{{"data":[{{"id":1,"modId":{id},"displayName":"1.0","fileName":"{slug}.jar",
                 "releaseType":1,"fileDate":"2026-01-01T00:00:00Z",
                 "downloadUrl":"https://exemple.invalid/{slug}.jar","fileLength":1,
                 "hashes":[],"dependencies":[]}}]}}"#
        ),
    );
}

/// Le même projet servi par le site, sans clé.
fn publier_web(serveur: &mc_essais::Serveur, id: u32, slug: &str) {
    serveur.json(
        &format!("/widget/{slug}"),
        &format!(r#"{{"id":{id},"title":"{slug}"}}"#),
    );
    serveur.json(
        &format!("/web/mods/{id}/files"),
        &format!(
            r#"{{"data":[{{"id":2,"fileName":"{slug}-web.jar","displayName":"1.0",
                 "fileLength":1,"releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
                 "gameVersions":["1.21.1","NeoForge"]}}],"pagination":{{"totalCount":1}}}}"#
        ),
    );
    serveur.json(&format!("/web/mods/{id}/dependencies"), r#"{"data":[]}"#);
}

/// Avec une clé qui marche, c'est la Core API qui répond : elle publie les
/// empreintes et le drapeau de redistribution, que le site ne donne pas.
#[tokio::test]
async fn avec_une_cle_valable_la_core_api_repond() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-core");
    publier_core(&serveur, 42, "jei");
    publier_web(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .curseforge_any("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1);
    assert_eq!(trouves[0].file_name, "jei.jar");
}

/// Une clé refusée ne doit pas tout arrêter : elle expire, elle se révoque, et
/// le mode sans clé reste capable d'installer.
#[tokio::test]
async fn une_cle_refusee_bascule_sur_l_acces_sans_cle() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-repli");
    serveur.code("/mods/search", 403);
    publier_web(&serveur, 42, "jei");

    let registre = registre(&atelier, &serveur, Some("$2a$10$revoquee"));
    let trouves = registre.curseforge_any("jei", MC, LOADER).await.unwrap();

    assert_eq!(trouves.len(), 1);
    assert_eq!(trouves[0].file_name, "jei-web.jar");

    // L'avertissement n'est émis qu'une fois par exécution — répété à chaque
    // mod, il deviendrait du bruit qu'on cesse de lire. Le second appel ne
    // retente donc même pas la Core API.
    let avant = serveur.appels("/mods/search");
    registre.curseforge_any("jei", MC, LOADER).await.unwrap();
    assert_eq!(serveur.appels("/mods/search"), avant);
}

/// Une panne ordinaire n'est pas un refus de clé : elle doit remonter, au lieu
/// de faire basculer en silence sur une source qui ne publie pas d'empreinte.
#[tokio::test]
async fn une_panne_de_la_core_api_remonte_au_lieu_de_basculer() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-panne");
    serveur.code("/mods/search", 500);
    publier_web(&serveur, 42, "jei");

    let erreur = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .curseforge_any("jei", MC, LOADER)
        .await
        .expect_err("la panne doit remonter");

    assert!(format!("{erreur:#}").contains("500"), "{erreur:#}");
}

/// Sans clé du tout, le site est la seule voie.
#[tokio::test]
async fn sans_cle_seul_le_site_est_consulte() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-sans-cle");
    publier_core(&serveur, 42, "jei");
    publier_web(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur, None)
        .curseforge_any("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].file_name, "jei-web.jar");
    assert_eq!(serveur.appels("/mods/search"), 0);
}

/// Modrinth est consultée avant CurseForge pour un `modId` : elle publie les
/// empreintes et la répartition client/serveur.
#[tokio::test]
async fn modrinth_passe_avant_curseforge_pour_un_modid() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-modid");
    let contenu = jar("bookshelf", &[]);
    serveur.octets("/bookshelf.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("bookshelf").version(Version::nouvelle(
            "20.2.0",
            &serveur.url("/bookshelf.jar"),
            &contenu,
        )),
    );
    publier_core(&serveur, 42, "bookshelf");

    let trouves = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].origin, crate::Origin::Modrinth);
    assert_eq!(serveur.appels("/mods/search"), 0);
}

/// Sans clé, la recherche par mot-clé est fermée : un `modId` inconnu partout
/// ne donne rien, et ce n'est pas une erreur.
#[tokio::test]
async fn un_modid_inconnu_de_toutes_les_sources_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-inconnu");

    let trouves = registre(&atelier, &serveur, None)
        .find_by_mod_id("mod-fantome", MC, LOADER)
        .await
        .unwrap();

    assert!(trouves.is_empty());
}

/// Un build épinglé chez CurseForge passe par la Core API quand la clé marche,
/// et par le site sinon.
#[tokio::test]
async fn un_build_epingle_bascule_aussi_sur_le_site() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-epingle");
    serveur.code("/mods/files", 403);
    serveur.json("/widget/jei", r#"{"id":42,"title":"JEI"}"#);
    serveur.json(
        "/web/mods/42/files/2",
        r#"{"id":2,"fileName":"jei-web.jar","displayName":"1.0","fileLength":1,
            "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
            "gameVersions":["1.21.1","NeoForge"]}"#,
    );

    let trouve = registre(&atelier, &serveur, Some("$2a$10$revoquee"))
        .curseforge_file("jei", "2")
        .await
        .unwrap()
        .expect("le site répond");

    assert_eq!(trouve.file_name, "jei-web.jar");
}

/// Une clé qui marche peut ne rien trouver : le projet n'est pas dans la Core
/// API, ou son auteur en a retiré la redistribution. Une réponse vide n'est pas
/// une réponse — il reste le site, qui connaît d'autres projets. La rendre
/// telle quelle laisserait le mod introuvable alors qu'il est publié.
#[tokio::test]
async fn une_reponse_vide_de_la_core_api_fait_consulter_le_site() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-vide");
    serveur.json("/mods/search", r#"{"data":[]}"#);
    publier_web(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .curseforge_any("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    assert_eq!(trouves[0].file_name, "jei-web.jar");
    // La Core API a bien été interrogée : c'est sa réponse vide qu'on écarte,
    // pas la clé qu'on ignore.
    assert_eq!(serveur.appels("/mods/search"), 1);
}

/// Même règle pour la recherche par `modId`, sur les deux sources. Modrinth
/// répond d'abord ; si elle ne connaît pas le mod, c'est la Core API, puis le
/// site. Chaque réponse vide passe la main à la suivante.
#[tokio::test]
async fn une_reponse_vide_passe_la_main_a_la_source_suivante() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-modid-vide");

    // Modrinth ne connaît pas ce modId.
    serveur.json("/v2/search", r#"{"hits":[]}"#);
    // La Core API non plus.
    serveur.json("/mods/search", r#"{"data":[]}"#);
    // Le site, si.
    serveur.json("/widget/bookshelf", r#"{"id":42,"title":"Bookshelf"}"#);
    publier_web(&serveur, 42, "bookshelf");

    let trouves = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    assert_eq!(trouves[0].file_name, "bookshelf-web.jar");
}

/// Ce que Modrinth trouve l'emporte, et la Core API n'est même pas consultée —
/// pas seulement parce qu'elle est plus lente, mais parce que Modrinth publie
/// les empreintes et la répartition client/serveur qu'on ne veut pas perdre.
#[tokio::test]
async fn ce_que_modrinth_trouve_n_est_pas_ecrase_par_curseforge() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-modrinth-prime");
    let contenu = jar("bookshelf", &[]);
    serveur.octets("/bookshelf.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("bookshelf").version(Version::nouvelle(
            "20.2.0",
            &serveur.url("/bookshelf.jar"),
            &contenu,
        )),
    );
    // CurseForge propose autre chose pour le même modId : si la réponse de
    // Modrinth était écartée, c'est ce fichier-ci qui ressortirait.
    publier_core(&serveur, 42, "bookshelf");
    publier_web(&serveur, 42, "bookshelf");

    let trouves = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].origin, crate::Origin::Modrinth);
    assert_ne!(trouves[0].file_name, "bookshelf-web.jar");
    assert_eq!(serveur.appels("/mods/search"), 0);
}

/// Un refus de clé sur la recherche par `modId` bascule sur le site, et se
/// retient : la clé ne redeviendra pas valable au milieu d'une résolution, et
/// la retenter pour chacun des cent mods d'un pack ajouterait cent
/// allers-retours perdus.
#[tokio::test]
async fn un_refus_de_cle_sur_un_modid_bascule_et_se_retient() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-modid-refus");
    serveur.json("/v2/search", r#"{"hits":[]}"#);
    serveur.code("/mods/search", 403);
    serveur.json("/widget/bookshelf", r#"{"id":42,"title":"Bookshelf"}"#);
    publier_web(&serveur, 42, "bookshelf");

    let registre = registre(&atelier, &serveur, Some("$2a$10$revoquee"));
    let trouves = registre
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .expect("le site prend le relais");
    assert_eq!(trouves[0].file_name, "bookshelf-web.jar");

    let avant = serveur.appels("/mods/search");
    registre.find_by_mod_id("bookshelf", MC, LOADER).await.ok();
    assert_eq!(
        serveur.appels("/mods/search"),
        avant,
        "la clé refusée est retentée"
    );
}

/// Une panne ordinaire n'est pas un refus de clé, ici non plus : elle remonte.
/// Basculer en silence sur le site ferait perdre les empreintes que seule la
/// Core API publie, pour une indisponibilité de quelques minutes.
#[tokio::test]
async fn une_panne_sur_un_modid_remonte_au_lieu_de_basculer() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-modid-panne");
    serveur.json("/v2/search", r#"{"hits":[]}"#);
    serveur.code("/mods/search", 500);

    let erreur = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .expect_err("la panne doit remonter");
    assert!(format!("{erreur:#}").contains("500"), "{erreur:#}");
}

/// Mêmes deux règles pour un build épinglé : le refus de clé se retient, la
/// panne remonte.
#[tokio::test]
async fn un_refus_de_cle_sur_un_build_epingle_se_retient() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-epingle-refus");
    serveur.code("/mods/files", 403);
    serveur.json("/widget/jei", r#"{"id":42,"title":"JEI"}"#);
    serveur.json(
        "/web/mods/42/files/2",
        r#"{"id":2,"fileName":"jei-web.jar","displayName":"1.0","fileLength":1,
            "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
            "gameVersions":["1.21.1","NeoForge"]}"#,
    );

    let registre = registre(&atelier, &serveur, Some("$2a$10$revoquee"));
    registre
        .curseforge_file("jei", "2")
        .await
        .unwrap()
        .expect("le site répond");

    let avant = serveur.appels("/mods/files");
    registre.curseforge_file("jei", "2").await.ok();
    assert_eq!(
        serveur.appels("/mods/files"),
        avant,
        "la clé refusée est retentée"
    );
}

/// Quand Modrinth ne connaît pas le mod mais que la Core API le trouve, c'est
/// elle qui répond — et non le site. Les deux publient le même projet, mais
/// seule la Core API donne l'empreinte du fichier ; s'en passer ferait
/// télécharger un jar qu'on ne peut plus vérifier.
#[tokio::test]
async fn ce_que_la_core_api_trouve_n_est_pas_ecrase_par_le_site() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-modid-core");
    serveur.json("/v2/search", r#"{"hits":[]}"#);
    publier_core(&serveur, 42, "bookshelf");
    publier_web(&serveur, 42, "bookshelf");

    let trouves = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    assert_eq!(
        trouves[0].file_name, "bookshelf.jar",
        "c'est la réponse du site qui est remontée"
    );
}

#[tokio::test]
async fn une_panne_sur_un_build_epingle_remonte_au_lieu_de_basculer() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-epingle-panne");
    serveur.code("/mods/files", 500);

    let erreur = registre(&atelier, &serveur, Some("$2a$10$cle"))
        .curseforge_file("jei", "2")
        .await
        .expect_err("la panne doit remonter");
    assert!(format!("{erreur:#}").contains("500"), "{erreur:#}");
}
