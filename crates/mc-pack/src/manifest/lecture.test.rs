use crate::manifest::essais::avec_serveurs;
use crate::manifest::*;


#[test]
fn un_pack_plus_recent_que_le_binaire_reste_installable() {
    // Le jour où mc-content déclare un environnement de plus, les binaires
    // déjà chez les joueurs ne le connaîtront pas. S'ils refusaient le
    // manifeste pour autant, une ligne ajoutée au pack couperait
    // l'installation de tout le parc d'un coup — et personne ne pourrait
    // plus rien télécharger pour se réparer.
    let brut = br#"{"schema":1,"name":"essai","minecraft":"1.21.1",
                    "loader":{"type":"neoforge","version":"latest"},
                    "servers":{"production":{"host":"mc.ggy.info"},
                               "qualification":{"host":"mc-qa.ggy.info"}},
                    "nouveau_champ_inconnu":true}"#;
    let manifest = Manifest::parse(brut).expect("un environnement inconnu n'est pas une faute");
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info".into()),
        "ce que ce binaire sait lire reste lisible"
    );
    assert!(
        manifest
            .server_for(mc_log::Environment::Development)
            .is_none()
    );
}

#[test]
fn les_serveurs_survivent_a_un_passage_par_le_cache() {
    // launch lit le manifeste rangé dans le cache, pas celui du réseau :
    // un champ perdu à l'écriture ferait s'ouvrir le jeu sur le menu au
    // lieu de rejoindre le serveur, et seulement hors ligne.
    let dossier = std::env::temp_dir().join(format!("mc-pack-essai-{}", std::process::id()));
    std::fs::create_dir_all(&dossier).unwrap();
    let chemin = dossier.join("samflix.json");
    avec_serveurs().save(&chemin).unwrap();
    let relu = Manifest::load(&chemin).unwrap();
    std::fs::remove_dir_all(&dossier).ok();
    assert_eq!(
        relu.server_for(mc_log::Environment::Development)
            .map(Server::address),
        Some("78.46.100.5:25566".into())
    );
}
