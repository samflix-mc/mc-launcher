use super::{Layout, PathBuf};

#[test]
fn les_instances_ne_partagent_que_le_commun() {
    let layout = Layout::new(PathBuf::from("/data"));
    let une = layout.instance("samflix");
    let autre = layout.instance("essai");

    assert_eq!(
        une.game_dir,
        PathBuf::from("/data/instances/samflix/minecraft")
    );
    assert_eq!(
        une.mods_dir(),
        PathBuf::from("/data/instances/samflix/minecraft/mods")
    );
    assert_ne!(une.mods_dir(), autre.mods_dir());
    // Bibliothèques et assets, eux, sont communs.
    assert_eq!(layout.shared(), PathBuf::from("/data/shared"));
}

/// Les quatre répertoires du launcher tiennent sous une seule racine : un seul
/// endroit à supprimer pour repartir de zéro.
#[test]
fn tout_tient_sous_une_seule_racine() {
    let layout = Layout::new(PathBuf::from("/data"));

    assert_eq!(layout.runtime(), PathBuf::from("/data/runtime"));
    assert_eq!(layout.cache(), PathBuf::from("/data/cache"));
    for chemin in [
        layout.shared(),
        layout.runtime(),
        layout.cache(),
        layout.instance("samflix").dir,
    ] {
        assert!(chemin.starts_with("/data"), "{}", chemin.display());
    }
}

#[test]
fn la_disposition_par_defaut_suit_le_repertoire_de_donnees() {
    assert_eq!(Layout::default().root, mc_dl::data_dir());
}

#[test]
fn une_instance_neuve_recoit_ses_trois_repertoires() {
    let arbre = crate::essais::Arbre::neuf("disposition");
    let layout = Layout::new(arbre.racine.clone());
    let instance = layout.instance("samflix");

    instance.create().expect("les répertoires se créent");

    assert!(instance.game_dir.is_dir());
    assert!(instance.mods_dir().is_dir());
    assert!(instance.config_dir().is_dir());
    assert_eq!(instance.name, "samflix");
}

/// Une racine impossible à créer doit se dire, avec le chemin fautif : c'est
/// un disque plein ou un montage en lecture seule, et le message est tout ce
/// dont dispose celui qui lit.
#[test]
fn un_repertoire_impossible_nomme_le_chemin_fautif() {
    let arbre = crate::essais::Arbre::neuf("disposition-bloquee");
    let obstacle = arbre.racine.join("instances");
    std::fs::write(&obstacle, b"pas un repertoire").unwrap();

    let erreur = Layout::new(arbre.racine.clone())
        .instance("samflix")
        .create()
        .expect_err("un fichier barre la route");

    assert!(format!("{erreur:#}").contains("samflix"), "{erreur:#}");
}
