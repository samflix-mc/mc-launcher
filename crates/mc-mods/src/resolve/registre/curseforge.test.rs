//! L'ordre des sources, et ce qu'on fait quand l'une ne répond pas.
//!
//! La Core API de CurseForge n'est plus interrogée : ce qui se vérifiait de sa
//! bascule — clé refusée, clé absente, repli — n'a plus d'objet. Reste
//! l'essentiel, qui n'a pas changé : Modrinth d'abord, le site ensuite, et une
//! absence qui n'est pas une panne.

use crate::essais::{Atelier, Projet, Version, jar, publier};
use crate::resolve::registre::Registry;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn registre(atelier: &Atelier, serveur: &mc_essais::Serveur) -> Registry {
    Registry::pour_essais(atelier.racine.join("cache"), &serveur.base()).unwrap()
}

/// Un projet servi par l'API publique du site.
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

#[tokio::test]
async fn le_site_est_la_seule_voie_chez_curseforge() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-site");
    publier_web(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur)
        .curseforge_any("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].file_name, "jei-web.jar");
}

/// Modrinth est consultée avant CurseForge pour un `modId` : elle publie les
/// empreintes et la répartition client/serveur, que le site ne donne pas.
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
    publier_web(&serveur, 42, "bookshelf");

    let trouves = registre(&atelier, &serveur)
        .find_by_mod_id("bookshelf", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].origin, crate::Origin::Modrinth);
    // Le site n'a pas été dérangé : Modrinth avait la réponse.
    assert_eq!(serveur.appels("/widget/bookshelf"), 0);
}

/// La recherche par mot-clé du site est fermée : un `modId` que Modrinth ne
/// connaît pas et qui n'est pas le slug d'un projet ne donne rien — et ce
/// n'est pas une erreur, la boucle de rattrapage doit pouvoir conclure.
#[tokio::test]
async fn un_modid_inconnu_de_toutes_les_sources_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-inconnu");

    let trouves = registre(&atelier, &serveur)
        .find_by_mod_id("mod-fantome", MC, LOADER)
        .await
        .unwrap();

    assert!(trouves.is_empty());
}

/// Modrinth qui ne connaît pas le projet passe la main, elle ne conclut pas.
#[tokio::test]
async fn une_reponse_vide_passe_la_main_a_la_source_suivante() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-vide");
    serveur.code("/project/jei", 404);
    publier_web(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur)
        .find_by_mod_id("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].origin, crate::Origin::CurseForge);
}

/// Un build épinglé par le verrou passe par le site, enveloppe comprise.
#[tokio::test]
async fn un_build_epingle_passe_par_le_site() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-epingle");
    serveur.json("/widget/jei", r#"{"id":42,"title":"JEI"}"#);
    serveur.json(
        "/web/mods/42/files/2",
        r#"{"data":{"id":2,"fileName":"jei-web.jar","displayName":"1.0","fileLength":1,
            "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
            "gameVersions":["1.21.1","NeoForge"]}}"#,
    );

    let trouve = registre(&atelier, &serveur)
        .curseforge_file("jei", "2")
        .await
        .unwrap()
        .expect("le build existe");

    assert_eq!(trouve.file_name, "jei-web.jar");
}

/// Un build épinglé qui n'existe plus ne donne rien : c'est l'appelant qui
/// décide que c'est une erreur, avec le nom du mod sous les yeux.
#[tokio::test]
async fn un_build_epingle_disparu_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("registre-epingle-absent");
    serveur.json("/widget/jei", r#"{"id":42,"title":"JEI"}"#);
    serveur.code("/web/mods/42/files/9999", 404);

    assert!(
        registre(&atelier, &serveur)
            .curseforge_file("jei", "9999")
            .await
            .unwrap()
            .is_none()
    );
}
