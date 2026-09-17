use super::load_remote;
use crate::essais::{Atelier, MANIFESTE, entree, verrou};
use crate::lockfile::Lockfile;
use crate::manifest::Manifest;

fn client() -> mc_dl::Downloader {
    mc_dl::Downloader::new(mc_dl::USER_AGENT).unwrap()
}

fn json_verrou() -> String {
    serde_json::to_string(&verrou(vec![entree("jei", "both", None)])).unwrap()
}

/// Le manifeste et le verrou sont récupérés ensemble, et le verrou est rejoué
/// tel quel : un joueur ne résout rien, sinon sa machine choisirait ses propres
/// versions et il arriverait sur le serveur avec des registres qui ne
/// concordent plus.
#[tokio::test]
async fn un_pack_distant_est_telecharge_avec_son_verrou_et_mis_en_cache() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("distant-nominal");
    serveur.json("/pack/samflix.json", MANIFESTE);
    serveur.json("/pack/samflix.lock.json", &json_verrou());

    let cache = atelier.racine.join("cache");
    let pack = load_remote(&serveur.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect("le pack se récupère");

    assert_eq!(pack.manifest.name, "samflix");
    assert!(pack.replay, "le verrou distant fait foi");
    assert!(!pack.from_cache);
    // La copie est laissée pour les lancements hors ligne.
    assert!(cache.join("samflix.json").is_file());
    assert!(cache.join("samflix.lock.json").is_file());
}

/// Jouer avec le pack d'hier vaut mieux que ne pas jouer.
#[tokio::test]
async fn hors_ligne_la_derniere_copie_connue_prend_le_relais() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("distant-cache");
    let cache = atelier.racine.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    Manifest::parse(MANIFESTE.as_bytes())
        .unwrap()
        .save(&cache.join("samflix.json"))
        .unwrap();
    verrou(vec![entree("jei", "both", None)])
        .save(&cache.join("samflix.lock.json"))
        .unwrap();

    // Le serveur ne sert rien : c'est la panne de réseau.
    serveur.code("/pack/samflix.json", 500);

    let pack = load_remote(&serveur.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect("la copie locale sauve le lancement");

    assert!(pack.from_cache, "le repli n'est pas signalé");
    assert!(pack.replay);
    assert_eq!(pack.manifest.name, "samflix");
}

/// Rien n'est mis en cache avant d'avoir été relu : une page d'erreur HTML
/// servie en HTTP 200 remplacerait sinon un pack valide par du vide.
#[tokio::test]
async fn une_reponse_illisible_ne_remplace_pas_la_copie_valide() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("distant-html");
    let cache = atelier.racine.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    Manifest::parse(MANIFESTE.as_bytes())
        .unwrap()
        .save(&cache.join("samflix.json"))
        .unwrap();
    verrou(vec![entree("jei", "both", None)])
        .save(&cache.join("samflix.lock.json"))
        .unwrap();

    serveur.json("/pack/samflix.json", "<html><body>503</body></html>");
    serveur.json("/pack/samflix.lock.json", &json_verrou());

    let pack = load_remote(&serveur.url("/pack/samflix.json"), &cache, &client())
        .await
        .unwrap();

    assert!(pack.from_cache);
    // La copie valide est intacte : elle n'a pas été écrasée par le HTML.
    let relu = Lockfile::load(&cache.join("samflix.lock.json")).unwrap();
    assert_eq!(relu.mods.len(), 1);
}

/// Un manifeste neuf accompagné du verrou d'avant décrirait un pack que
/// personne n'a jamais publié : les deux ou aucun.
#[tokio::test]
async fn un_verrou_illisible_annule_aussi_le_manifeste() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("distant-verrou-casse");
    let cache = atelier.racine.join("cache");
    serveur.json("/pack/samflix.json", MANIFESTE);
    serveur.json("/pack/samflix.lock.json", "pas un verrou");

    let erreur = load_remote(&serveur.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect_err("ni pack utilisable, ni copie");

    assert!(format!("{erreur:#}").contains("aucune copie"), "{erreur:#}");
    assert!(
        !cache.join("samflix.json").exists(),
        "un manifeste orphelin a été mis en cache"
    );
}

#[tokio::test]
async fn sans_reseau_ni_copie_l_echec_nomme_le_cache() {
    let serveur = mc_essais::Serveur::neuf().await;
    let atelier = Atelier::neuf("distant-rien");
    let cache = atelier.racine.join("cache");
    serveur.code("/pack/samflix.json", 404);

    let erreur = load_remote(&serveur.url("/pack/samflix.json"), &cache, &client())
        .await
        .expect_err("rien nulle part");

    let texte = format!("{erreur:#}");
    assert!(texte.contains("inutilisable"), "{texte}");
    assert!(texte.contains("cache"), "{texte}");
}
