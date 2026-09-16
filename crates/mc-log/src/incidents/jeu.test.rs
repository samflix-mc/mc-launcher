use super::truncate;

#[test]
fn un_extrait_se_tronque_sans_couper_un_caractere() {
    // Un journal de jeu est plein d'accents : trancher au milieu d'un
    // caractère ferait paniquer le rapport de plantage lui-même.
    let texte = "é".repeat(200);
    let borne = truncate(&texte, 101);
    assert!(borne.ends_with('é'));
    // `strip_prefix` et non `trim_start_matches` : celui-ci ne fait rien
    // quand le marqueur manque, si bien que l'assertion passait encore le
    // jour où la troncature cessait d'en poser un.
    let extrait = borne
        .strip_prefix("[…début tronqué…]\n")
        .expect("marqueur de troncature absent");
    assert!(texte.ends_with(extrait));
}
