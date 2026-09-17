use crate::essais::{Projet, Version, jar, publier};
use crate::modrinth::Modrinth;
use std::sync::Arc;

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

fn client(serveur: &mc_essais::Serveur) -> Modrinth {
    let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap());
    Modrinth::avec_base(dl, &serveur.base())
}

/// La recherche par `modId` parcourt les résultats par ordre de pertinence et
/// s'arrête au premier projet qui a effectivement une version compatible.
///
/// S'arrêter au premier venu, compatible ou non, rendrait une liste vide dès
/// qu'un projet homonyme — abandonné, ou publié pour une autre version du jeu —
/// arrive en tête. La dépendance serait déclarée introuvable alors qu'elle est
/// juste en deuxième position, et le pack s'installerait sans elle.
#[tokio::test]
async fn la_recherche_par_modid_passe_au_resultat_suivant() {
    let serveur = mc_essais::Serveur::neuf().await;

    let contenu = jar("bookshelf", &[]);
    serveur.octets("/bookshelf.jar", &contenu);
    // Le second projet a une version ; le premier n'en a aucune.
    publier(&serveur, &Projet::nouveau("abandonne"));
    publier(
        &serveur,
        &Projet::nouveau("bookshelf").version(Version::nouvelle(
            "20.2.0",
            &serveur.url("/bookshelf.jar"),
            &contenu,
        )),
    );
    serveur.json(
        "/search",
        r#"{"hits":[{"slug":"abandonne","project_id":"abandonne-id","title":"Abandonné"},
                    {"slug":"bookshelf","project_id":"bookshelf-id","title":"Bookshelf"}]}"#,
    );

    // Le modId n'est pas le slug du projet — c'est le cas qui oblige à passer
    // par la recherche, et donc à parcourir ses résultats.
    let trouves = client(&serveur)
        .find_by_mod_id("bookshelflib", MC, LOADER)
        .await
        .unwrap();

    assert_eq!(trouves.len(), 1, "{trouves:?}");
    assert_eq!(trouves[0].slug, "bookshelf");
}

/// Aucun résultat n'est pas une erreur : un `modId` lu dans un jar peut ne
/// correspondre à aucun projet publié — une bibliothèque embarquée, un mod
/// retiré. L'appelant a d'autres sources à consulter.
#[tokio::test]
async fn une_recherche_sans_resultat_ne_leve_pas_d_erreur() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/search", r#"{"hits":[]}"#);

    let trouves = client(&serveur)
        .find_by_mod_id("mod-fantome", MC, LOADER)
        .await
        .expect("rien trouvé n'est pas une panne");
    assert!(trouves.is_empty());
}
