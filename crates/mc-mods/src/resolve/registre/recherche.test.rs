use crate::essais::{Atelier, Projet, Version, jar, publier};
use crate::resolve::demande::Request;
use crate::resolve::registre::Registry;
use crate::{Origin, jar::Side};

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn registre(atelier: &Atelier, serveur: &mc_essais::Serveur) -> Registry {
    Registry::pour_essais(
        atelier.racine.join("cache"),
        &serveur.base(),
        Some("$2a$10$cle"),
    )
    .unwrap()
}

/// Un projet CurseForge, servi par la Core API avec son fichier.
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
            r#"{{"data":[{{"id":7,"modId":{id},"displayName":"1.0","fileName":"{slug}-core.jar",
                 "releaseType":1,"fileDate":"2026-01-01T00:00:00Z",
                 "downloadUrl":"https://exemple.invalid/{slug}.jar","fileLength":1,
                 "gameVersions":["1.21.1","NeoForge"],"hashes":[],"dependencies":[]}}],
                 "pagination":{{"totalCount":1}}}}"#
        ),
    );
    serveur.json(&format!("/mods/{id}"), &format!(r#"{{"data":{{"id":{id},"name":"{slug}","slug":"{slug}","links":{{"websiteUrl":"https://exemple.invalid/{slug}"}},"allowModDistribution":true}}}}"#));
    serveur.json(
        &format!("/mods/{id}/files/7"),
        &format!(
            r#"{{"data":{{"id":7,"modId":{id},"displayName":"1.0","fileName":"{slug}-core.jar",
             "releaseType":1,"fileDate":"2026-01-01T00:00:00Z",
             "downloadUrl":"https://exemple.invalid/{slug}.jar","fileLength":1,
             "gameVersions":["1.21.1","NeoForge"],"hashes":[],"dependencies":[]}}}}"#
        ),
    );
}

/// Un identifiant numérique ne peut venir que de CurseForge : Modrinth nomme
/// ses projets par des slugs. Le lui proposer ferait une requête vouée à
/// l'échec pour chaque dépendance résolue — cent mods, cent allers-retours.
#[tokio::test]
async fn un_identifiant_numerique_n_est_pas_propose_a_modrinth() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-numerique");
    publier_core(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur)
        .candidates("42", None, MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].file_name, "jei-core.jar");
    assert_eq!(
        serveur.appels("/project/42"),
        0,
        "Modrinth a été interrogée pour un identifiant numérique"
    );
}

/// Quand la source est nommée, elle est la seule consultée. Un manifeste qui
/// écrit « modrinth » a une raison de le faire — l'empreinte, la licence, la
/// répartition client/serveur — et retomber sur CurseForge en silence lui
/// donnerait un autre fichier que celui qu'il a demandé.
#[tokio::test]
async fn une_source_nommee_n_est_pas_doublee_par_l_autre() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-source");
    // Modrinth ne connaît pas ce projet ; CurseForge, si.
    publier_core(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur)
        .candidates("jei", Some(Origin::Modrinth), MC, LOADER)
        .await
        .unwrap();

    assert!(
        trouves.is_empty(),
        "CurseForge a répondu pour une demande adressée à Modrinth : {trouves:?}"
    );
    assert_eq!(serveur.appels("/mods/search"), 0);
}

/// Sans source nommée, Modrinth passe d'abord ; si elle ne trouve rien, c'est
/// CurseForge qui répond.
#[tokio::test]
async fn sans_source_nommee_modrinth_passe_avant_curseforge() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-defaut");
    let contenu = jar("jei", &[]);
    serveur.octets("/jei.jar", &contenu);
    publier(
        &serveur,
        &Projet::nouveau("jei").version(Version::nouvelle(
            "19.21",
            &serveur.url("/jei.jar"),
            &contenu,
        )),
    );
    publier_core(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur)
        .candidates("jei", None, MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].origin, Origin::Modrinth);
    assert_eq!(serveur.appels("/mods/search"), 0);
}

/// Un build épinglé par un identifiant numérique s'adresse à CurseForge sans
/// passer par Modrinth : le numéro ne veut rien dire pour elle.
#[tokio::test]
async fn un_build_epingle_numerique_s_adresse_a_curseforge() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-epingle-num");
    publier_core(&serveur, 42, "jei");
    // Un build isolé se demande par la recherche d'empreintes : la route du
    // fichier seul exigerait aussi l'identifiant du projet.
    serveur.json(
        "/mods/files",
        r#"{"data":[{"id":7,"modId":42,"displayName":"1.0","fileName":"jei-core.jar",
             "releaseType":1,"fileDate":"2026-01-01T00:00:00Z",
             "downloadUrl":"https://exemple.invalid/jei.jar","fileLength":1,
             "gameVersions":["1.21.1","NeoForge"],"hashes":[],"dependencies":[]}]}"#,
    );

    let mut demande = Request::new("42");
    demande.file = Some("7".into());
    demande.side = Some(Side::Both);

    let trouve = registre(&atelier, &serveur)
        .pinned(&demande, MC, LOADER)
        .await
        .expect("la Core API répond")
        .expect("le build épinglé existe");

    assert_eq!(trouve.file_name, "jei-core.jar");
    assert_eq!(
        serveur.appels("/version/7"),
        0,
        "Modrinth a été interrogée pour un build numérique"
    );
}

/// L'autre moitié de la règle : une demande adressée à CurseForge ne passe pas
/// par Modrinth non plus, fût-elle nommée par un slug. Les deux conditions
/// comptent chacune pour elle-même.
#[tokio::test]
async fn un_slug_adresse_a_curseforge_ne_passe_pas_par_modrinth() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-slug-cf");
    publier_core(&serveur, 42, "jei");

    let trouves = registre(&atelier, &serveur)
        .candidates("jei", Some(Origin::CurseForge), MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves[0].file_name, "jei-core.jar");
    assert_eq!(
        serveur.appels("/project/jei"),
        0,
        "Modrinth a été interrogée pour une demande adressée à CurseForge"
    );
}

/// Un build épinglé par un identifiant **non** numérique vient de Modrinth :
/// c'est elle qui nomme ses versions ainsi. L'envoyer à CurseForge ferait
/// chercher un numéro de fichier là où il n'y a qu'un slug, et l'épinglage
/// serait déclaré introuvable alors qu'il existe.
#[tokio::test]
async fn un_build_epingle_non_numerique_s_adresse_a_modrinth() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-epingle-slug");

    let mut demande = Request::new("jei");
    demande.file = Some("eyZ2YBGT".into());
    demande.side = Some(Side::Both);

    // Aucune source ne connaît ce build : ce qui se vérifie ici, c'est à qui
    // la question est posée.
    let _ = registre(&atelier, &serveur)
        .pinned(&demande, MC, LOADER)
        .await;

    assert_eq!(
        serveur.appels("/version/eyZ2YBGT"),
        1,
        "Modrinth n'a pas été consultée pour un build qu'elle seule nomme ainsi"
    );
}

/// Sans build épinglé, il n'y a rien à chercher : la demande suit le cours
/// ordinaire. Rendre `None` pour un build qui *est* épinglé ferait retomber sur
/// la dernière version — exactement ce que l'épinglage sert à empêcher.
#[tokio::test]
async fn une_demande_sans_epinglage_ne_cherche_pas_de_build() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("recherche-sans-epingle");

    let trouve = registre(&atelier, &serveur)
        .pinned(&Request::new("jei"), MC, LOADER)
        .await
        .unwrap();

    assert!(trouve.is_none());
    assert_eq!(serveur.appels("/mods/files"), 0);
}
