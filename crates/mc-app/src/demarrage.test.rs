use super::{DELAI_DE_GARDE, PASSAGE_FAIT};
use std::sync::atomic::Ordering;

/// Le délai de garde est plus long que tout démarrage réel, et plus court que
/// ce qu'un joueur regarderait sans comprendre.
///
/// Le borner des deux côtés a un sens : trop court, la garde couperait un
/// démarrage lent au premier lancement et montrerait une fenêtre à moitié
/// rendue ; trop long, une erreur JavaScript laisserait quelqu'un devant un
/// écran figé sans savoir qu'il peut tuer le processus.
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
/// Ce test ne vaut que parce qu'il est le seul de ce binaire à lire ce
/// drapeau : `passer_la_main` demande un `AppHandle`, qui demande une
/// application Tauri, qui demande un serveur d'affichage. L'idempotence, elle,
/// s'éprouve en lançant le launcher — voir la recette de `docs/interface.md`.
#[test]
fn le_passage_n_est_pas_fait_au_depart() {
    assert!(!PASSAGE_FAIT.load(Ordering::Acquire));
}
