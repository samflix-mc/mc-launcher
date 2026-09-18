use super::http::Requete;
use super::scenario::Etat;
use super::{Contexte, argument, router};

fn requete(chemin: &str) -> Requete {
    Requete {
        methode: "POST".to_string(),
        chemin: chemin.to_string(),
        corps: String::new(),
    }
}

/// **Le contrat du serveur : les mêmes noms que ceux du pont.**
///
/// Si un nom diverge, le front reçoit un 404 pour une commande qui existe —
/// et le symptôme est un écran vide sans message, puisque `invoke` et `fetch`
/// échouent silencieusement de la même façon.
#[tokio::test]
async fn toutes_les_commandes_du_pont_repondent() {
    let contexte = Contexte::neuf(Etat::PretAJouer);

    for nom in [
        "marque",
        "chemin",
        "statut",
        "etat_du_pack",
        "nouvelles",
        "reglages",
        "verifier_les_fichiers",
        "ecran",
        "ouvrir_dossier",
        "front_pret",
    ] {
        let reponse = router(contexte.clone(), requete(&format!("/commande/{nom}"))).await;
        assert_eq!(reponse.code, 200, "{nom} : {}", reponse.corps);
    }
}

#[tokio::test]
async fn une_commande_inconnue_le_dit() {
    let contexte = Contexte::neuf(Etat::PretAJouer);
    let reponse = router(contexte, requete("/commande/plante")).await;

    assert_eq!(reponse.code, 404);
    assert!(reponse.corps.contains("plante"), "{}", reponse.corps);
}

/// Le scénario change en une requête, et la réponse suivante en tient compte.
/// C'est tout l'intérêt du serveur : atteindre un état sans avoir à le
/// provoquer pour de bon.
#[tokio::test]
async fn le_scenario_se_change_et_la_suite_le_suit() {
    let contexte = Contexte::neuf(Etat::Deconnecte);

    let avant = router(contexte.clone(), requete("/commande/statut")).await;
    assert_eq!(avant.corps, "null", "personne n'est connecté au départ");

    let bascule = router(contexte.clone(), requete("/scenario/pret-a-jouer")).await;
    assert_eq!(bascule.code, 200);

    let apres = router(contexte.clone(), requete("/commande/statut")).await;
    assert!(apres.corps.contains("thesam1798"), "{}", apres.corps);
}

#[tokio::test]
async fn un_scenario_inconnu_le_dit() {
    let contexte = Contexte::neuf(Etat::Deconnecte);
    let reponse = router(contexte, requete("/scenario/n-importe-quoi")).await;

    assert_eq!(reponse.code, 404);
}

/// Chaque scénario doit pouvoir être demandé par son nom : un nom dans la
/// liste que `depuis_nom` ne reconnaît pas serait un scénario annoncé et
/// inatteignable.
#[tokio::test]
async fn chaque_scenario_annonce_est_atteignable() {
    let contexte = Contexte::neuf(Etat::Deconnecte);

    for (nom, attendu) in Etat::TOUS {
        let reponse = router(contexte.clone(), requete(&format!("/scenario/{nom}"))).await;
        assert_eq!(reponse.code, 200, "{nom}");
        assert_eq!(
            *contexte.etat.lock().expect("état"),
            attendu,
            "{nom} n'a pas posé l'état qu'il annonce"
        );
    }
}

/// Le compte SANS LICENCE est un état à part entière, pas une absence de
/// compte : c'est celui que la page Connexion doit savoir afficher.
#[tokio::test]
async fn sans_licence_rend_un_compte_qui_ne_possede_pas_le_jeu() {
    let contexte = Contexte::neuf(Etat::SansLicence);
    let reponse = router(contexte, requete("/commande/statut")).await;

    assert!(
        reponse.corps.contains("\"possedeLeJeu\":false"),
        "{}",
        reponse.corps
    );
}

/// La racine annonce ce qu'on peut demander. C'est la documentation qu'on lit
/// quand on a oublié les noms — donc elle doit être exacte.
#[tokio::test]
async fn la_racine_annonce_les_scenarios_et_les_commandes() {
    let contexte = Contexte::neuf(Etat::HorsLigne);
    let reponse = router(contexte, requete("/")).await;

    assert_eq!(reponse.code, 200);
    assert!(reponse.corps.contains("hors-ligne"), "{}", reponse.corps);
    assert!(reponse.corps.contains("etat_du_pack"));
}

/// Les arguments arrivent comme `invoke` les envoie : un objet dont les clés
/// sont les noms des paramètres.
#[test]
fn un_argument_nomme_se_lit_dans_le_corps() {
    assert_eq!(
        argument(r#"{"profond":true}"#, "profond"),
        Some(serde_json::Value::Bool(true))
    );
    assert_eq!(argument(r#"{"autre":1}"#, "profond"), None);
    assert_eq!(argument("pas du json", "profond"), None);
}

/// Le fil n'est vide QUE hors ligne : ailleurs, la page des nouvelles doit
/// avoir du contenu à montrer — c'est la seule façon de la regarder tant que
/// mc-content n'a rien publié.
#[tokio::test]
async fn le_fil_porte_des_billets_sauf_hors_ligne() {
    let avec = Contexte::neuf(Etat::PretAJouer);
    let reponse = router(avec, requete("/commande/nouvelles")).await;
    assert!(
        reponse.corps.contains("Le launcher est là"),
        "{}",
        reponse.corps
    );

    let sans = Contexte::neuf(Etat::HorsLigne);
    let reponse = router(sans, requete("/commande/nouvelles")).await;
    assert!(
        reponse.corps.contains("\"billets\":[]"),
        "{}",
        reponse.corps
    );
}

/// Le corps des billets est un ARBRE, jamais du HTML : c'est la condition à
/// laquelle le CSP a été desserré, et elle vaut aussi pour les données de
/// démonstration.
#[tokio::test]
async fn les_billets_de_demonstration_sont_des_arbres() {
    let contexte = Contexte::neuf(Etat::PretAJouer);
    let reponse = router(contexte, requete("/commande/nouvelles")).await;

    assert!(
        reponse.corps.contains("\"type\":\"paragraphe\""),
        "{}",
        reponse.corps
    );
    assert!(
        !reponse.corps.contains("<p>"),
        "du HTML a fui dans le corps"
    );
}
