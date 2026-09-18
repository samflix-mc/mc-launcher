use super::{ETIQUETTE, EVENEMENT_SESSION, HAUTEUR, LARGEUR, ROUTE, connexion_en_cours};

/// **Les dimensions viennent du design system, et elles sont vérifiées.**
///
/// 440 × 520 est ce que sa fiche AuthWindow pose. Un nombre recopié de travers
/// ne se voit pas dans une revue — c'est une fenêtre un peu trop étroite, et
/// l'on cherche ensuite pourquoi le libellé du bouton Microsoft passe à la
/// ligne.
#[test]
fn la_fenetre_a_les_dimensions_du_design_system() {
    assert_eq!((LARGEUR, HAUTEUR), (440.0, 520.0));
}

/// L'étiquette est écrite des DEUX côtés : ici, et dans `capabilities`, sans
/// quoi les boutons de la barre de titre de cette fenêtre-là sont refusés par
/// l'ACL — et ce refus ne se voit qu'en build empaqueté.
#[test]
fn l_etiquette_est_celle_que_la_capacite_declare() {
    let capacite = include_str!("../capabilities/default.json");

    assert_eq!(ETIQUETTE, "connexion");
    assert!(
        capacite.contains("\"connexion\""),
        "capabilities/default.json ne couvre pas la fenêtre « {ETIQUETTE} »"
    );
}

/// La route chargée est une route du ROUTEUR, pas un fichier.
///
/// Elle s'appuie sur le repli du protocole d'actifs, qui sert `index.html` pour
/// un chemin inconnu. L'écrire `/connexion.html` marcherait aussi — et
/// chargerait un fichier qui n'existe pas, donc une fenêtre blanche.
#[test]
fn la_route_est_celle_du_routeur() {
    assert_eq!(ROUTE, "/connexion");
    assert!(!ROUTE.ends_with(".html"));
}

/// Au repos, aucune connexion n'est en cours : c'est ce qui autorise
/// `demarrage::accomplir` à montrer la fenêtre principale.
#[test]
fn au_repos_aucune_connexion_n_est_en_cours() {
    assert!(!connexion_en_cours());
}

/// Le nom de l'événement est un contrat avec le front, écrit des deux côtés.
#[test]
fn l_evenement_de_session_porte_le_nom_que_le_front_ecoute() {
    assert_eq!(EVENEMENT_SESSION, "session-ouverte");
}
