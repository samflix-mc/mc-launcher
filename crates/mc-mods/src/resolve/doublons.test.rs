use super::{Origin, Reason, deduplicate_by_mod_id};
use crate::resolve::essais::{embarquant, installed, map};

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
    let ecartes = deduplicate_by_mod_id(&mut chosen);

    assert_eq!(chosen.len(), 1);
    // Celui qui porte une empreinte est conservé : il est vérifiable.
    assert!(chosen.values().next().unwrap().candidate.sha1.is_some());

    // Ce qui est retiré est nommé, avec le gagnant et le modId en cause.
    assert_eq!(ecartes.len(), 1);
    assert_eq!(ecartes[0].ecarte, "jade-cf");
    assert_eq!(ecartes[0].retenu, "jade");
    assert_eq!(ecartes[0].mod_id, "jade");
    // Une dépendance écartée n'est pas une contradiction du manifeste.
    assert!(!ecartes[0].explicite);
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
    assert!(deduplicate_by_mod_id(&mut chosen).is_empty());
    assert_eq!(chosen.len(), 2);
}

/// Le bug qui a fait retirer Sodium, Iris et EntityCulling du pack samflix.
///
/// Sodium et Iris embarquent les mêmes quatre shims Fabric ; EntityCulling et
/// Not Enough Animations les mêmes libs de tr7zw. Ce sont des apports, pas des
/// identités : NeoForge dédup̀lique les jars embarqués au chargement, et un
/// pack qui garde Iris sans Sodium est cassé.
#[test]
fn deux_mods_qui_embarquent_la_meme_bibliotheque_restent_tous_les_deux() {
    let mut chosen = map(vec![
        embarquant(
            installed("sodium", &["sodium"], &[]),
            &[
                "fabric_api_base",
                "fabric_block_view_api_v2",
                "fabric_renderer_api_v1",
            ],
        ),
        embarquant(
            installed("iris", &["iris"], &[]),
            &[
                "fabric_api_base",
                "fabric_block_view_api_v2",
                "fabric_renderer_api_v1",
            ],
        ),
    ]);

    assert!(deduplicate_by_mod_id(&mut chosen).is_empty());
    assert_eq!(chosen.len(), 2, "un mod légitime a été supprimé");
}

/// La frontière est bien entre racine et embarqué, et non entre « premier » et
/// « second » : un mod qui embarque ce qu'un autre déclare comme sien reste
/// distinct de lui.
#[test]
fn un_modid_embarque_ne_prend_pas_la_place_du_mod_qui_le_declare() {
    let mut chosen = map(vec![
        // La bibliothèque installée pour elle-même.
        installed("cloth-config", &["cloth_config"], &[]),
        // Un mod qui embarque la même bibliothèque.
        embarquant(
            installed("architectury", &["architectury"], &[]),
            &["cloth_config"],
        ),
    ]);

    assert!(deduplicate_by_mod_id(&mut chosen).is_empty());
    assert_eq!(chosen.len(), 2);
}

/// Un mod demandé au manifeste et écarté est signalé comme tel : c'est ce qui
/// permet à l'appelant de refuser plutôt que de rendre un verrou incomplet.
#[test]
fn un_mod_explicite_ecarte_est_marque_comme_tel() {
    let mut premier = installed("jade", &["jade"], &[]);
    premier.candidate.sha1 = Some("aa".into());
    let mut second = installed("jade-miroir", &["jade"], &[]);
    second.candidate.sha1 = None;

    let mut chosen = map(vec![premier, second]);
    let ecartes = deduplicate_by_mod_id(&mut chosen);

    assert_eq!(ecartes.len(), 1);
    assert_eq!(ecartes[0].ecarte, "jade-miroir");
    assert!(ecartes[0].explicite, "les deux venaient du manifeste");
}
