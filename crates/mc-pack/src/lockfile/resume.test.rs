use super::Side;
use crate::lockfile::lecture::tests::{lock, locked};
use mc_mods::Origin;

#[test]
fn le_diff_dit_ce_qui_a_bouge() {
    let avant = lock(vec![
        locked("jei", "a", "19.51"),
        locked("jade", "b", "15.10"),
    ]);
    let apres = lock(vec![
        locked("jei", "c", "19.56"),
        locked("bookshelf-lib", "d", "21.1.81"),
    ]);

    let lignes = apres.diff(&avant);
    assert!(lignes.contains(&"~ jei 19.51 → 19.56".to_string()));
    assert!(lignes.contains(&"+ bookshelf-lib 21.1.81".to_string()));
    assert!(lignes.contains(&"- jade 15.10".to_string()));
}

#[test]
fn rejouer_un_verrou_epingle_chaque_build() {
    let verrou = lock(vec![locked("jei", "9myHusbW", "19.56")]);
    let requests = verrou.requests();
    assert_eq!(requests[0].file.as_deref(), Some("9myHusbW"));
    assert_eq!(requests[0].source, Some(Origin::Modrinth));
    assert_eq!(requests[0].side, Some(Side::Both));
}

/// La génération qui bouge se dit en toutes lettres, avec sa conséquence.
///
/// C'est la seule ligne du diff qui décrive un EFFET et non un fait : celui
/// qui publie doit voir, dans la revue de la PR, qu'il vient de demander à
/// chaque joueur d'effacer ses mods. Un « 0 → 1 » perdu entre trente lignes de
/// versions ne le lui dirait pas.
#[test]
fn une_generation_qui_bouge_se_dit_avec_sa_consequence() {
    let mut avant = lock(Vec::new());
    avant.generation = 0;
    let mut apres = lock(Vec::new());
    apres.generation = 1;

    let lignes = apres.diff(&avant);

    assert_eq!(lignes.len(), 1, "{lignes:?}");
    assert!(lignes[0].contains("génération 0 → 1"), "{lignes:?}");
    assert!(lignes[0].contains("effaceront"), "{lignes:?}");
    // La contrepartie doit être dite dans la même phrase : sans elle, la ligne
    // se lit comme « on efface tout », et personne n'ose publier.
    assert!(lignes[0].contains("sauvegardes"), "{lignes:?}");
}

/// À génération constante, le diff n'en parle pas — il ne dit que ce qui a
/// bougé.
#[test]
fn une_generation_inchangee_ne_dit_rien() {
    let avant = lock(Vec::new());
    let apres = lock(Vec::new());
    assert!(apres.diff(&avant).is_empty());
}
