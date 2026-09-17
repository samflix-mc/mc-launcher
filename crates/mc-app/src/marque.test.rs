use super::*;

#[test]
fn un_nom_d_un_seul_mot_donne_ses_deux_premieres_lettres() {
    assert_eq!(sceau("Prism"), "PR");
    assert_eq!(sceau("samflix"), "SA");
}

#[test]
fn un_nom_de_plusieurs_mots_donne_ses_initiales() {
    assert_eq!(sceau("Mon Réseau"), "MR");
    assert_eq!(sceau("Le Grand Serveur"), "LG");
}

#[test]
fn un_tiret_separe_deux_mots_comme_une_espace() {
    // « samflix-mc » se lit comme deux mots, et « SM » l'abrège mieux que
    // « SA », qui perdrait la seconde moitié du nom.
    assert_eq!(sceau("samflix-mc"), "SM");
    assert_eq!(sceau("mon_reseau"), "MR");
}

#[test]
fn un_nom_sans_lettre_ne_donne_pas_un_rond_vide() {
    // Improbable, mais un sceau vide se remarque plus qu'un point
    // d'interrogation, et se diagnostique moins bien.
    assert_eq!(sceau(""), "??");
    assert_eq!(sceau("— —"), "??");
}

#[test]
fn un_nom_d_une_seule_lettre_reste_affichable() {
    assert_eq!(sceau("X"), "X");
}

#[test]
fn le_nom_par_defaut_est_celui_du_reseau() {
    // La compilation sans `MC_LAUNCHER_NOM` doit réussir et retomber ici.
    // Quand la variable est posée, c'est elle qui gagne — ce que ce test ne
    // peut pas vérifier, puisqu'elle est lue à la compilation.
    assert!(!nom().is_empty());
}

#[test]
fn la_marque_se_serialise_avec_ses_deux_champs() {
    let json = serde_json::to_value(Marque::courante()).expect("sérialisation");

    assert!(json["nom"].is_string());
    assert_eq!(json["sceau"].as_str().map(str::len), Some(2));
}
