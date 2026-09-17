use super::ajoutes_par_dependance;
use mc_mods::Reason;

/// C'est le chiffre qui explique qu'un manifeste de trente lignes installe
/// cent mods : le reste vient des dépendances. Compter les autres — ceux que
/// le manifeste nomme — annoncerait le contraire, et ferait croire à une
/// résolution qui n'a rien trouvé alors qu'elle a tout trouvé.
#[test]
fn seuls_les_mods_non_demandes_comptent_comme_ajoutes() {
    let raisons = [
        Reason::Explicit,
        Reason::Declared {
            by: "jei".to_string(),
        },
        Reason::Implicit {
            by: "jei".to_string(),
            mod_id: "bookshelf".to_string(),
        },
    ];

    assert_eq!(ajoutes_par_dependance(raisons.iter()), 2);

    // Un pack dont le manifeste nomme tout n'a rien gagné en chemin.
    assert_eq!(ajoutes_par_dependance([Reason::Explicit].iter()), 0);
    // Et un pack vide n'a rien ajouté non plus.
    assert_eq!(ajoutes_par_dependance([].iter()), 0);
}
