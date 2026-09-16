use super::*;
use crate::resolve::essais::candidat;
use crate::Origin;

/// déclaré, on trouve qui le fournit, et sa demande rejoint la file.
#[test]
fn un_fournisseur_trouve_devient_une_demande_implicite() {
    let suite = suite_du_rattrapage(
        Some(candidat("bookshelf-lib", "21.1.81")),
        &BTreeSet::new(),
        "bookshelf".into(),
        "attributefix".into(),
        Side::Both,
    );
    match suite {
        Rattrapage::Demander(request, reason) => {
            assert_eq!(request.slug, "bookshelf-lib-id");
            assert_eq!(request.source, Some(Origin::Modrinth));
            assert_eq!(request.file.as_deref(), Some("bookshelf-lib-21.1.81"));
            assert_eq!(
                reason,
                Reason::Implicit {
                    by: "attributefix".into(),
                    mod_id: "bookshelf".into()
                }
            );
        }
        autre => panic!("attendu une demande, obtenu {autre:?}"),
    }
}

/// Le cas qui faisait tourner la résolution jusqu'à MAX_PASSES : le seul
/// fournisseur occupe une clé qu'une demande plus autoritaire tient déjà.
/// Redemander ne changerait rien, donc on consigne le manque.
#[test]
fn un_fournisseur_en_impasse_est_consigne_au_lieu_d_etre_redemande() {
    let mut impasses = BTreeSet::new();
    impasses.insert((Origin::Modrinth, "bookshelf-lib-id".to_string()));
    let suite = suite_du_rattrapage(
        Some(candidat("bookshelf-lib", "21.1.81")),
        &impasses,
        "bookshelf".into(),
        "attributefix".into(),
        Side::Both,
    );
    assert_eq!(
        suite,
        Rattrapage::Renoncer(Unresolved {
            mod_id: "bookshelf".into(),
            required_by: "attributefix".into(),
            side: Side::Both,
        })
    );
}

#[test]
fn un_modid_que_personne_ne_fournit_est_consigne() {
    let suite = suite_du_rattrapage(
        None,
        &BTreeSet::new(),
        "libfoo".into(),
        "build_b".into(),
        Side::Client,
    );
    assert_eq!(
        suite,
        Rattrapage::Renoncer(Unresolved {
            mod_id: "libfoo".into(),
            required_by: "build_b".into(),
            side: Side::Client,
        })
    );
}
