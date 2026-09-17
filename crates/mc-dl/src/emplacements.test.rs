use super::data_dir;

/// Tout ce que le launcher installe vit sous une seule racine : un seul
/// endroit à supprimer pour repartir de zéro, et rien qui traîne dans le
/// répertoire personnel.
#[test]
fn les_donnees_vivent_sous_un_seul_repertoire_nomme() {
    assert!(data_dir().ends_with("samflix-mc"), "{:?}", data_dir());
    assert!(data_dir().is_absolute() || data_dir().starts_with("."));
}

/// `XDG_DATA_HOME` prime quand il est posé — c'est la convention du système,
/// et un poste qui la suit ne doit pas se retrouver avec deux emplacements.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn la_convention_du_systeme_est_respectee() {
    // SAFETY : les deux variables sont restaurées avant la fin du test, et
    // aucun autre test de ce crate ne les lit.
    let xdg = std::env::var_os("XDG_DATA_HOME");

    unsafe {
        std::env::set_var("XDG_DATA_HOME", "/ailleurs/partage");
    }
    assert_eq!(
        data_dir(),
        std::path::PathBuf::from("/ailleurs/partage/samflix-mc")
    );

    // Une variable posée mais vide ne désigne rien : la traiter comme une
    // racine mettrait les données à « /samflix-mc ».
    unsafe {
        std::env::set_var("XDG_DATA_HOME", "");
    }
    assert!(data_dir().ends_with("share/samflix-mc"), "{:?}", data_dir());

    unsafe {
        match xdg {
            Some(valeur) => std::env::set_var("XDG_DATA_HOME", valeur),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }
}
