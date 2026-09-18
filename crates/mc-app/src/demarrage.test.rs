use super::{DELAI_DE_GARDE, DUREE_MINIMALE, PASSAGE_FAIT, reste_a_attendre};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Un front plus rapide que le plancher est RETENU, exactement le temps qui
/// manque.
///
/// C'est le cas qui a motivé la règle : mesuré sur le poste de Sam, le front
/// signalait au bout d'environ six cents millisecondes, et l'écran de démarrage
/// passait trop vite pour être lu — il se remarquait comme un clignotement, et
/// non comme un démarrage.
#[test]
fn un_front_rapide_est_retenu_du_temps_qui_manque() {
    assert_eq!(
        reste_a_attendre(Duration::from_millis(600)),
        Duration::from_millis(900)
    );
    assert_eq!(
        reste_a_attendre(Duration::from_millis(100)),
        Duration::from_millis(1400)
    );
}

/// Un front plus lent que le plancher n'attend PAS.
///
/// Ajouter le plancher au temps réel ferait d'un démarrage déjà lent un
/// démarrage plus lent encore — et c'est précisément sur les machines lentes
/// qu'on ne veut rien ajouter.
#[test]
fn un_front_lent_n_attend_pas() {
    assert!(reste_a_attendre(DUREE_MINIMALE).is_zero());
    assert!(reste_a_attendre(Duration::from_secs(3)).is_zero());
    // Et surtout : pas de débordement. `saturating_sub` et non une
    // soustraction, qui paniquerait sur `Duration` dès que l'écoulé dépasse le
    // plancher — c'est-à-dire sur toutes les machines lentes, et seulement
    // sur elles.
    assert!(reste_a_attendre(Duration::from_secs(600)).is_zero());
}

/// À la borne exacte, on n'attend rien de plus.
///
/// Le test de la borne ± 1, que les conventions du dépôt exigent de toute
/// comparaison : sans lui, un `<` devenu `<=` — ou l'inverse — survivrait.
#[test]
fn a_la_borne_exacte_et_autour() {
    assert_eq!(
        reste_a_attendre(DUREE_MINIMALE - Duration::from_millis(1)),
        Duration::from_millis(1)
    );
    assert!(reste_a_attendre(DUREE_MINIMALE).is_zero());
    assert!(reste_a_attendre(DUREE_MINIMALE + Duration::from_millis(1)).is_zero());
}

/// Les deux bornes ne se croisent pas.
///
/// Un plancher plus long que le plafond ferait attendre la garde pour rien, et
/// la fenêtre s'afficherait après le délai censé la sauver. C'est absurde, et
/// c'est le genre d'absurdité qu'un réglage de confort finit par produire.
#[test]
fn le_plancher_reste_sous_le_plafond() {
    assert!(
        DUREE_MINIMALE < DELAI_DE_GARDE,
        "le plancher de l'écran de démarrage dépasse le délai de garde"
    );
}

/// Le plancher est borné des deux côtés, et pour deux raisons opposées.
#[test]
fn le_plancher_est_borne_des_deux_cotes() {
    assert!(
        DUREE_MINIMALE >= Duration::from_millis(800),
        "trop court : l'écran repasserait pour un clignotement"
    );
    assert!(
        DUREE_MINIMALE <= Duration::from_secs(3),
        "trop long : on ferait attendre pour le plaisir de faire attendre"
    );
}

/// Le délai de garde est plus long que tout démarrage réel, et plus court que
/// ce qu'un joueur regarderait sans comprendre.
#[test]
fn le_delai_de_garde_est_borne_des_deux_cotes() {
    assert!(
        DELAI_DE_GARDE.as_secs() >= 5,
        "trop court : un premier lancement sur disque lent serait coupé"
    );
    assert!(
        DELAI_DE_GARDE.as_secs() <= 20,
        "trop long : personne ne regarde un écran figé vingt secondes"
    );
}

/// Le drapeau part à faux : sans cela, le tout premier passage serait ignoré
/// et la fenêtre ne s'afficherait jamais.
///
/// Ce test ne vaut que parce qu'il est le seul de ce binaire à le lire :
/// `accomplir` demande un `AppHandle`, qui demande une application Tauri, qui
/// demande un serveur d'affichage. Le reste s'éprouve en lançant le launcher —
/// voir la recette de `docs/interface.md`.
#[test]
fn le_passage_n_est_pas_fait_au_depart() {
    assert!(!PASSAGE_FAIT.load(Ordering::Acquire));
}
