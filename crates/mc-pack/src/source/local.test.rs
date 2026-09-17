use super::super::Source;
use crate::essais::{Atelier, MANIFESTE, entree, verrou};
use crate::manifest::Manifest;

fn layout(atelier: &Atelier) -> mc_instance::Layout {
    mc_instance::Layout::new(atelier.racine.clone())
}

/// Un pack distant déjà installé se relit depuis sa copie, sans réseau : c'est
/// exactement le moment où l'on veut jouer.
#[test]
fn un_pack_distant_installe_se_relit_depuis_sa_copie() {
    let atelier = Atelier::neuf("local-cache");
    let layout = layout(&atelier);
    let url = "https://mc-launcher.ggy.info/pack/samflix.json";
    let source = Source::parse(url, &layout);

    let Source::Remote { cache_dir, .. } = &source else {
        panic!("une adresse donne une source distante");
    };
    std::fs::create_dir_all(cache_dir).unwrap();
    Manifest::parse(MANIFESTE.as_bytes())
        .unwrap()
        .save(&cache_dir.join("samflix.json"))
        .unwrap();
    verrou(vec![entree("jei", "both", None)])
        .save(&cache_dir.join("samflix.lock.json"))
        .unwrap();

    let pack = source.load_local().expect("la copie est là");

    assert_eq!(pack.manifest.name, "samflix");
    // Le verrou distant fait foi : ses builds sont rejoués, pas cherchés.
    assert!(pack.replay);
    assert_eq!(pack.lock.unwrap().mods.len(), 1);
}

/// Le cache est rangé par hôte : à plat, passer de la dev à la production
/// écraserait silencieusement la première, et une panne de réseau ressortirait
/// le pack du mauvais environnement.
#[test]
fn les_copies_de_deux_environnements_ne_se_marchent_pas_dessus() {
    let atelier = Atelier::neuf("local-hotes");
    let layout = layout(&atelier);

    let production = Source::parse("https://mc-launcher.ggy.info/pack/samflix.json", &layout);
    let dev = Source::parse(
        "https://mc-launcher-dev.ggy.info/pack/samflix.json",
        &layout,
    );

    let (Source::Remote { cache_dir: a, .. }, Source::Remote { cache_dir: b, .. }) =
        (&production, &dev)
    else {
        panic!("deux sources distantes");
    };
    assert_ne!(a, b, "les deux environnements partagent le même cache");
}

/// `lock` a besoin d'un fichier à réécrire : résoudre un pack distant n'aurait
/// nulle part où poser son résultat.
#[test]
fn un_pack_distant_n_a_pas_de_chemin_a_editer() {
    let atelier = Atelier::neuf("local-chemin");
    let layout = layout(&atelier);

    assert!(
        Source::parse("https://exemple.invalid/p.json", &layout)
            .local_path()
            .is_none()
    );
    assert!(
        Source::parse("packs/samflix.json", &layout)
            .local_path()
            .is_some()
    );
}
