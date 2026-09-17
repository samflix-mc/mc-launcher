use super::super::Source;
use crate::essais::{Atelier, MANIFESTE, entree, verrou};

fn layout(atelier: &Atelier) -> mc_instance::Layout {
    mc_instance::Layout::new(atelier.racine.clone())
}

fn client() -> mc_dl::Downloader {
    mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap()
}

/// Une adresse se reconnaît à son protocole ; tout le reste est un chemin.
/// Sans cette règle, installer depuis le dépôt et installer depuis le site
/// demanderaient deux commandes pour le même geste.
#[test]
fn une_adresse_se_distingue_d_un_chemin_par_son_protocole() {
    let atelier = Atelier::neuf("source-parse");
    let layout = layout(&atelier);

    for adresse in [
        "https://mc-launcher.ggy.info/pack/samflix.json",
        "HTTP://exemple.invalid/p.json",
    ] {
        let source = Source::parse(adresse, &layout);
        assert!(source.is_remote(), "« {adresse} » devrait être une adresse");
        assert_eq!(source.describe(), adresse);
        assert!(source.local_path().is_none());
    }

    for chemin in ["packs/samflix.json", "./samflix.json", "/tmp/p.json"] {
        let source = Source::parse(chemin, &layout);
        assert!(!source.is_remote(), "« {chemin} » devrait être un chemin");
        assert_eq!(source.local_path().unwrap(), std::path::Path::new(chemin));
    }
}

#[tokio::test]
async fn un_pack_local_se_lit_avec_son_verrou() {
    let atelier = Atelier::neuf("source-fichier");
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());
    verrou(vec![entree("jei", "both", None)])
        .save(&atelier.racine.join("samflix.lock.json"))
        .unwrap();

    let pack = Source::parse(manifeste.to_str().unwrap(), &layout(&atelier))
        .load(&client())
        .await
        .expect("le pack se lit");

    assert_eq!(pack.manifest.name, "samflix");
    assert_eq!(pack.lock.as_ref().unwrap().mods.len(), 1);
    // Un pack local se résout : ses builds sont cherchés, pas rejoués.
    assert!(!pack.replay);
    assert!(!pack.from_cache);
}

/// La première fois qu'un pack local est résolu, il n'y a pas de verrou — et
/// ce n'est pas une erreur.
#[tokio::test]
async fn un_pack_local_sans_verrou_se_lit_quand_meme() {
    let atelier = Atelier::neuf("source-sans-verrou");
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());

    let pack = Source::parse(manifeste.to_str().unwrap(), &layout(&atelier))
        .load(&client())
        .await
        .unwrap();

    assert!(pack.lock.is_none());
    assert!(pack.lock_path.ends_with("samflix.lock.json"));
}

/// `launch` et `verify` parlent de l'installation présente sur le disque :
/// aller rechercher le pack publié les ferait décrire un autre pack, et
/// lancer une partie cesserait de marcher sans réseau.
#[test]
fn la_lecture_locale_ne_touche_pas_au_reseau() {
    let atelier = Atelier::neuf("source-locale");
    let manifeste = atelier.ecrire("samflix.json", MANIFESTE.as_bytes());

    let pack = Source::parse(manifeste.to_str().unwrap(), &layout(&atelier))
        .load_local()
        .expect("le pack est là");

    assert_eq!(pack.manifest.name, "samflix");
    assert!(!pack.replay);
}

/// Un pack distant jamais installé ne peut pas être lu localement : le dire
/// vaut mieux que de renvoyer un manifeste vide.
#[test]
fn un_pack_distant_jamais_installe_se_dit() {
    let atelier = Atelier::neuf("source-jamais");
    let source = Source::parse("https://exemple.invalid/samflix.json", &layout(&atelier));

    let erreur = source.load_local().expect_err("rien en cache");
    assert!(
        format!("{erreur:#}").contains("mc-pack install"),
        "{erreur:#}"
    );
}

#[test]
fn un_manifeste_local_absent_se_dit_avec_son_chemin() {
    let atelier = Atelier::neuf("source-absent");
    let source = Source::parse(
        atelier.racine.join("nulle-part.json").to_str().unwrap(),
        &layout(&atelier),
    );

    let erreur = source.load_local().expect_err("rien à cette place");
    assert!(format!("{erreur:#}").contains("nulle-part"), "{erreur:#}");
}
