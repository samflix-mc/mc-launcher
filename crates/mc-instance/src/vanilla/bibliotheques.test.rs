use super::*;

fn artefact(size: u64) -> Artifact {
    Artifact {
        path: None,
        sha1: String::new(),
        size,
        url: String::new(),
    }
}

#[test]
fn le_poids_du_lot_est_la_somme_des_tailles_annoncees() {
    let retenues = vec![
        ("a/b/lwjgl.jar".to_string(), artefact(1_200)),
        ("c/d/asm.jar".to_string(), artefact(800)),
    ];

    assert_eq!(poids(&retenues), 2_000);
}

#[test]
fn un_lot_vide_ne_pese_rien() {
    // Toutes les bibliothèques peuvent être exclues par les règles de
    // plateforme. Une barre qui part d'un total nul ne doit pas être une
    // division par zéro plus loin — c'est ici que le cas se constate.
    assert_eq!(poids(&[]), 0);
}
