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
