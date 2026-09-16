use super::*;
use crate::resolve::essais::{installed, map};

#[test]
fn un_meme_mod_venu_de_deux_sources_n_est_garde_qu_une_fois() {
    // Demandé par son slug Modrinth, puis tiré comme dépendance par son
    // identifiant CurseForge : rien ne rapproche les deux clés de projet,
    // sauf le modId. Deux jars du même modId feraient échouer NeoForge.
    let mut depuis_modrinth = installed("jade", &["jade"], &[]);
    depuis_modrinth.candidate.sha1 = Some("aa".into());

    let mut depuis_cf = installed("jade-cf", &["jade"], &[]);
    depuis_cf.candidate.origin = Origin::CurseForge;
    depuis_cf.candidate.sha1 = None;
    depuis_cf.reason = Reason::Declared { by: "autre".into() };

    let mut chosen = map(vec![depuis_modrinth, depuis_cf]);
    deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    // Celui qui porte une empreinte est conservé : il est vérifiable.
    assert!(chosen.values().next().unwrap().candidate.sha1.is_some());
}

#[test]
fn a_empreinte_egale_le_mod_demande_l_emporte() {
    let mut explicite = installed("jade", &["jade"], &[]);
    explicite.candidate.sha1 = Some("aa".into());

    let mut dependance = installed("jade-bis", &["jade"], &[]);
    dependance.candidate.sha1 = Some("bb".into());
    dependance.reason = Reason::Declared { by: "autre".into() };

    let mut chosen = map(vec![explicite, dependance]);
    deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    assert_eq!(chosen.values().next().unwrap().reason, Reason::Explicit);
}

#[test]
fn deux_mods_distincts_ne_sont_pas_deduplicates() {
    let mut chosen = map(vec![
        installed("jei", &["jei"], &[]),
        installed("jade", &["jade"], &[]),
    ]);
    deduplicate_by_mod_id(&mut chosen);
    assert_eq!(chosen.len(), 2);
}
