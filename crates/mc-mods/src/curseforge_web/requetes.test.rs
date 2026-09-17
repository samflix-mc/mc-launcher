use super::super::CurseForgeWeb;
use std::sync::Arc;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

/// Un client dont les deux racines pointent vers le serveur de test.
fn client(serveur: &mc_essais::Serveur) -> CurseForgeWeb {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    CurseForgeWeb::avec_bases(dl, &serveur.base(), &serveur.url("/widget"))
}

/// Un fichier tel que la route du site le rend.
fn fichier(id: u64, nom: &str, versions: &str) -> String {
    format!(
        r#"{{"id":{id},"fileName":"{nom}","displayName":"{nom}","fileLength":1024,
             "releaseType":1,"dateCreated":"2026-01-01T00:00:00Z",
             "gameVersions":[{versions}]}}"#
    )
}

fn page(fichiers: &[String], total: usize) -> String {
    format!(
        r#"{{"data":[{}],"pagination":{{"totalCount":{total}}}}}"#,
        fichiers.join(",")
    )
}

#[tokio::test]
async fn un_slug_est_resolu_par_cfwidget_puis_ses_fichiers_listes() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/widget/jei",
        r#"{"id":238222,"title":"Just Enough Items"}"#,
    );
    serveur.json(
        "/mods/238222/files",
        &page(&[fichier(5001, "jei-19.jar", r#""1.21.1","NeoForge""#)], 1),
    );
    serveur.json("/mods/238222/dependencies", r#"{"data":[]}"#);

    let trouves = client(&serveur)
        .candidates("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    assert_eq!(trouves[0].name, "Just Enough Items");
    assert_eq!(trouves[0].project_id, "238222");
    // Aucune empreinte publiée par cette source : elle sera calculée au
    // téléchargement puis figée dans le verrou.
    assert!(trouves[0].sha1.is_none());
    // L'URL du CDN n'est pas reconstruite : c'est par cette reconstruction
    // qu'on contournerait le refus d'un auteur d'être redistribué.
    assert!(trouves[0].url.ends_with("/mods/238222/files/5001/download"));
}

/// `gameVersions` mélange versions, chargeurs et côtés. Un fichier Fabric ne
/// doit pas être servi à un pack NeoForge.
#[tokio::test]
async fn un_fichier_d_un_autre_chargeur_est_ecarte() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/jei", r#"{"id":1,"title":"JEI"}"#);
    serveur.json(
        "/mods/1/files",
        &page(
            &[
                fichier(1, "jei-fabric.jar", r#""1.21.1","Fabric""#),
                fichier(2, "jei-neo.jar", r#""1.21.1","NeoForge","Client""#),
            ],
            2,
        ),
    );
    serveur.json("/mods/1/dependencies", r#"{"data":[]}"#);

    let trouves = client(&serveur)
        .candidates("jei", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    assert_eq!(trouves[0].file_name, "jei-neo.jar");
}

/// La pagination est ignorée par le serveur et `pageSize` plafonné : un mod qui
/// a publié plus de cinquante fichiers depuis sa dernière version compatible
/// devient invisible. Le silence serait trompeur.
#[tokio::test]
async fn une_version_hors_de_la_fenetre_visible_est_signalee() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/vieuxmod", r#"{"id":7,"title":"Vieux Mod"}"#);
    serveur.json(
        "/mods/7/files",
        &page(
            &[fichier(1, "vieux-1.20.jar", r#""1.20.1","NeoForge""#)],
            400,
        ),
    );

    let erreur = client(&serveur)
        .candidates("vieuxmod", MC, LOADER)
        .await
        .expect_err("rien de compatible dans la fenêtre");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("400"), "{texte}");
    // La marche à suivre, et non une configuration qui n'existe plus.
    assert!(texte.contains("file"), "{texte}");
}

/// Rien trouvé et rien de plus à voir : c'est une absence ordinaire, pas une
/// erreur — l'appelant doit pouvoir continuer.
#[tokio::test]
async fn une_absence_ordinaire_ne_leve_pas_d_erreur() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/absent", r#"{"id":9,"title":"Absent"}"#);
    serveur.json("/mods/9/files", &page(&[], 0));
    serveur.json("/mods/9/dependencies", r#"{"data":[]}"#);

    let trouves = client(&serveur)
        .candidates("absent", MC, LOADER)
        .await
        .unwrap();
    assert!(trouves.is_empty());
}

/// Les dépendances du site sont déclarées par projet et non par fichier :
/// elles valent pour toutes les versions.
#[tokio::test]
async fn les_dependances_du_projet_sont_recopiees_sur_chaque_version() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/mod", r#"{"id":3,"title":"Mod"}"#);
    serveur.json(
        "/mods/3/files",
        &page(
            &[
                fichier(1, "a.jar", r#""1.21.1","NeoForge""#),
                fichier(2, "b.jar", r#""1.21.1","NeoForge""#),
            ],
            2,
        ),
    );
    serveur.json(
        "/mods/3/dependencies",
        r#"{"data":[{"id":42,"slug":"bookshelf","type":"RequiredDependency"},
                    {"id":43,"slug":"jei","type":"OptionalDependency"}]}"#,
    );

    let trouves = client(&serveur)
        .candidates("mod", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 2);
    for candidat in &trouves {
        // Seules les obligatoires comptent, et le slug est préféré à
        // l'identifiant : il permet de retrouver le projet sur Modrinth.
        assert_eq!(
            candidat.declared_deps.len(),
            1,
            "{:?}",
            candidat.declared_deps
        );
        assert_eq!(candidat.declared_deps[0].project_id, "bookshelf");
    }
}

/// Une dépendance sans slug ne laisse que l'identifiant numérique : il vaut
/// mieux le suivre que de perdre la dépendance.
#[tokio::test]
async fn une_dependance_sans_slug_retombe_sur_son_identifiant() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/mod", r#"{"id":3,"title":"Mod"}"#);
    serveur.json(
        "/mods/3/files",
        &page(&[fichier(1, "a.jar", r#""1.21.1","NeoForge""#)], 1),
    );
    serveur.json(
        "/mods/3/dependencies",
        r#"{"data":[{"id":42,"slug":"","type":"RequiredDependency"}]}"#,
    );

    let trouves = client(&serveur)
        .candidates("mod", MC, LOADER)
        .await
        .unwrap();
    assert_eq!(trouves[0].declared_deps[0].project_id, "42");
}

/// Un identifiant numérique vient d'une dépendance déjà résolue : le nom
/// lisible n'est pas indispensable, et l'économiser évite un appel à cfwidget,
/// service tiers bénévole.
#[tokio::test]
async fn un_identifiant_numerique_evite_l_appel_a_cfwidget() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json(
        "/mods/238222/files",
        &page(&[fichier(1, "a.jar", r#""1.21.1","NeoForge""#)], 1),
    );
    serveur.json("/mods/238222/dependencies", r#"{"data":[]}"#);

    let trouves = client(&serveur)
        .candidates("238222", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1);
    assert_eq!(serveur.appels("/widget/238222"), 0);
}

/// 403 est la réponse de Cloudflare comme celle d'une route fermée : dans les
/// deux cas, cette source n'a rien à offrir et l'appelant doit continuer.
#[tokio::test]
async fn un_refus_du_site_vaut_absence_et_non_echec() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/widget/jei", r#"{"id":1,"title":"JEI"}"#);
    serveur.code("/mods/1/files", 403);

    let trouves = client(&serveur)
        .candidates("jei", MC, LOADER)
        .await
        .unwrap();
    assert!(trouves.is_empty());
}

#[tokio::test]
async fn un_slug_inconnu_de_cfwidget_ne_donne_rien() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/widget/inconnu", 404);

    let trouves = client(&serveur)
        .candidates("inconnu", MC, LOADER)
        .await
        .unwrap();
    assert!(trouves.is_empty());
}
