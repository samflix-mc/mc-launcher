use super::*;

fn job(size: u64) -> Job {
    Job {
        key: (Origin::Modrinth, "jei".to_string()),
        url: String::new(),
        file_name: String::new(),
        sum: None,
        size,
    }
}

#[test]
fn le_poids_du_lot_est_la_somme_des_tailles_annoncees() {
    assert_eq!(poids(&[job(4_000_000), job(1_500_000)]), 5_500_000);
}

#[test]
fn un_mod_sans_taille_publiee_compte_pour_zero() {
    // CurseForge sans clé ne publie pas de taille. Le total devient un
    // plancher : la barre accélérera à la fin, ce qui vaut mieux que de
    // refuser d'en afficher une.
    assert_eq!(poids(&[job(0), job(2_000)]), 2_000);
}

#[test]
fn un_lot_vide_ne_pese_rien() {
    // Tout est déjà dans le cache : rien à descendre, et surtout pas de
    // division par zéro dans ce qui affichera un pourcentage.
    assert_eq!(poids(&[]), 0);
}
