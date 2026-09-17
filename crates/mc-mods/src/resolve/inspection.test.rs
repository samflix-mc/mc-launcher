use super::{Side, inspect_all, missing_requirements};
use crate::resolve::essais::{embarquant, installed, map};

#[test]
fn une_dependance_absente_est_signalee() {
    let chosen = map(vec![installed(
        "attributefix",
        &["attributefix"],
        &[("bookshelf", Side::Both)],
    )]);
    let missing = missing_requirements(&chosen);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "bookshelf");
    assert_eq!(missing[0].1, "attributefix");
}

#[test]
fn une_dependance_fournie_sous_un_autre_slug_ne_manque_pas() {
    // Le modId `bookshelf` est publié sous le slug `bookshelf-lib` : c'est
    // le modId du jar qui fait foi, jamais le nom du projet.
    let chosen = map(vec![
        installed(
            "attributefix",
            &["attributefix"],
            &[("bookshelf", Side::Both)],
        ),
        installed("bookshelf-lib", &["bookshelf"], &[]),
    ]);
    assert!(missing_requirements(&chosen).is_empty());
}

#[test]
fn une_dependance_embarquee_par_jarjar_ne_manque_pas() {
    // Le mod embarque sa bibliothèque : l'installer en plus donnerait deux
    // versions du même modId, ce que NeoForge refuse au chargement.
    //
    // Le modId embarqué satisfait donc l'exigence, alors même qu'il ne compte
    // pas comme identité du mod — c'est toute la distinction.
    let chosen = map(vec![embarquant(
        installed("un-mod", &["unmod"], &[("unelib", Side::Both)]),
        &["unelib"],
    )]);
    assert!(missing_requirements(&chosen).is_empty());
}

/// Ce qu'un autre mod embarque compte aussi : deux mods du même auteur se
/// partagent souvent une bibliothèque qu'un seul des deux emporte.
#[test]
fn une_dependance_embarquee_par_un_autre_mod_ne_manque_pas_non_plus() {
    let chosen = map(vec![
        installed(
            "attributefix",
            &["attributefix"],
            &[("bookshelf", Side::Both)],
        ),
        embarquant(installed("un-autre", &["autre"], &[]), &["bookshelf"]),
    ]);
    assert!(missing_requirements(&chosen).is_empty());
}

/// L'inspection ne lit que les jars fraîchement téléchargés : ceux qui
/// déclarent déjà ce qu'ils fournissent ont été lus, et ceux qui n'ont pas de
/// fichier n'existent pas encore sur le disque. Confondre les deux conditions
/// enverrait `jar::inspect` ouvrir un chemin vide — la résolution s'arrêterait
/// sur une erreur de lecture, au lieu de continuer son tour.
#[test]
fn une_entree_sans_fichier_n_est_pas_ouverte() {
    let mut chosen = map(vec![installed("pas-encore", &[], &[])]);
    for entree in chosen.values_mut() {
        // Rien de téléchargé : pas de chemin, et rien de déclaré.
        entree.path = std::path::PathBuf::new();
        entree.provides.clear();
    }

    inspect_all(&mut chosen).expect("une entrée sans fichier se saute");
}

#[test]
fn les_cotes_de_deux_demandeurs_sont_fusionnes() {
    let chosen = map(vec![
        installed("a", &["a"], &[("lib", Side::Client)]),
        installed("b", &["b"], &[("lib", Side::Server)]),
    ]);
    let missing = missing_requirements(&chosen);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].2, Side::Both);
}
