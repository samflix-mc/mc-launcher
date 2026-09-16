use super::*;
use crate::source::Source;


#[test]
fn une_adresse_se_reconnait_a_son_protocole() {
    assert!(is_url("https://mc-launcher.ggy.info/pack/samflix.json"));
    assert!(is_url("HTTP://exemple.invalid/x.json"));
    assert!(!is_url("packs/samflix.json"));
    assert!(!is_url("/var/tmp/samflix.json"));
}

#[test]
fn le_verrou_distant_se_deduit_du_manifeste() {
    assert_eq!(
        lock_url_for("https://exemple.invalid/pack/samflix.json"),
        "https://exemple.invalid/pack/samflix.lock.json"
    );
}

#[test]
fn le_nom_de_fichier_ignore_la_requete() {
    assert_eq!(
        file_name_of("https://exemple.invalid/pack/samflix.json?v=3"),
        "samflix.json"
    );
    assert_eq!(file_name_of("https://exemple.invalid/pack/"), "pack.json");
}

#[test]
fn deux_environnements_ne_partagent_pas_leur_cache() {
    let layout = mc_instance::Layout::new(PathBuf::from("/data"));
    let prod = cache_dir_for("https://mc-launcher.ggy.info/pack/samflix.json", &layout);
    let dev = cache_dir_for(
        "https://mc-launcher-dev.ggy.info/pack/samflix.json",
        &layout,
    );
    assert_ne!(prod, dev);
    assert!(prod.ends_with("mc-launcher.ggy.info"));
}

#[test]
fn un_chemin_reste_un_chemin() {
    let layout = mc_instance::Layout::new(PathBuf::from("/data"));
    let source = Source::parse("packs/samflix.json", &layout);
    assert!(!source.is_remote());
    assert_eq!(source.describe(), "packs/samflix.json");
}
