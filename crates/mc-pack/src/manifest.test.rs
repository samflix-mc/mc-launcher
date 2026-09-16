use super::*;

fn base() -> Manifest {
    Manifest {
        schema: SCHEMA,
        name: "essai".into(),
        version: None,
        minecraft: "1.21.1".into(),
        loader: Loader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: None,
        mods: Vec::new(),
        servers: BTreeMap::new(),
    }
}

#[test]
fn un_nom_qui_sort_du_repertoire_est_refuse() {
    // Le manifeste vient du réseau depuis que le pack distant est la
    // source par défaut, et « deploy » supprime les .jar du répertoire que
    // ce nom désigne.
    for fautif in [
        "../../../../home/sam/Documents",
        "/home/sam/.minecraft",
        "..",
        ".",
        "",
        "   ",
        "samflix/../..",
        r"..\..\Windows",
    ] {
        let mut manifest = base();
        manifest.name = fautif.into();
        assert!(
            manifest.check().is_err(),
            "« {fautif} » aurait dû être refusé"
        );
    }
}

#[test]
fn un_nom_inhabituel_mais_sans_danger_passe() {
    // Ce contrôle ne juge pas du bon goût. Refuser ici ce qui est
    // seulement inattendu condamnerait un pack futur chez tous les
    // launchers déjà distribués — et un launcher qui refuse le pack ne
    // peut plus se dépanner.
    for correct in ["samflix", "samflix v2", "pack.été-2026", "SAMFLIX_2"] {
        let mut manifest = base();
        manifest.name = correct.into();
        assert!(
            manifest.check().is_ok(),
            "« {correct} » aurait dû être accepté"
        );
    }
}

fn avec_serveurs() -> Manifest {
    let mut manifest = base();
    manifest.servers.insert(
        "development".into(),
        Server {
            host: "78.46.100.5".into(),
            port: Some(25566),
        },
    );
    manifest.servers.insert(
        "production".into(),
        Server {
            host: "mc.ggy.info".into(),
            port: Some(25565),
        },
    );
    manifest
}

#[test]
fn le_serveur_suit_l_environnement() {
    let manifest = avec_serveurs();
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
        Some("78.46.100.5:25566".into())
    );
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info:25565".into()),
        "un port déclaré s'écrit, fût-il le port par défaut"
    );
}

fn serveur(host: &str, port: Option<u16>) -> Server {
    Server {
        host: host.into(),
        port,
    }
}

#[test]
fn une_ipv6_est_mise_entre_crochets() {
    // Guava, dont Minecraft se sert pour lire l'adresse, refuse tout ce qui
    // porte plus d'un « : » sans crochets. Une IPv6 nue ne donnait donc pas
    // une mauvaise adresse : elle n'en donnait aucune, et le jeu s'ouvrait
    // sur le menu comme si le pack n'avait rien déclaré.
    assert_eq!(
        serveur("2001:db8::1", Some(25566)).address(),
        "[2001:db8::1]:25566"
    );
    assert_eq!(serveur("2001:db8::1", None).address(), "[2001:db8::1]");
    assert_eq!(
        serveur("[2001:db8::1]", Some(25566)).address(),
        "[2001:db8::1]:25566",
        "des crochets déjà posés ne se doublent pas"
    );
}

#[test]
fn un_nom_et_une_ipv4_restent_intacts() {
    assert_eq!(serveur("mc.ggy.info", None).address(), "mc.ggy.info");
    assert_eq!(
        serveur("78.46.100.5", Some(25566)).address(),
        "78.46.100.5:25566"
    );
}

#[test]
fn un_hote_deja_suffixe_d_un_port_est_signale() {
    // « mc.ggy.info:25566 » plus un champ « port » composerait
    // « mc.ggy.info:25566:25570 », que Minecraft refuse — et le refus est
    // muet, comme pour une IPv6 nue.
    let mut manifest = base();
    manifest.servers.insert(
        "production".into(),
        serveur("mc.ggy.info:25566", Some(25570)),
    );
    assert_eq!(manifest.problemes_de_serveurs().len(), 1);

    // Seul le doublon gêne : un hôte suffixé sans champ « port » compose
    // une adresse valable, et rien ne justifie de la refuser.
    let mut manifest = base();
    manifest
        .servers
        .insert("production".into(), serveur("mc.ggy.info:25566", None));
    assert!(manifest.problemes_de_serveurs().is_empty());
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info:25566".into())
    );
}

#[test]
fn un_binaire_local_rejoint_la_dev() {
    // Un binaire compilé à la main est un binaire de travail. Le faire
    // tomber sur la production reviendrait à envoyer quelqu'un qui essaie
    // là où d'autres jouent.
    let manifest = avec_serveurs();
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Local)
            .map(Server::address),
        manifest
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
    );
}

#[test]
fn une_preproduction_sans_serveur_ne_lance_rien() {
    // Elle n'a pas de serveurs Minecraft derrière elle, et ce n'est pas un
    // oubli : le nœud est unique, chaque réseau complet coûte de la RAM.
    assert!(
        avec_serveurs()
            .server_for(mc_log::Environment::Preproduction)
            .is_none()
    );
}

fn avec_cle(cle: &str, host: &str) -> Manifest {
    let mut manifest = base();
    manifest.servers.insert(
        cle.into(),
        Server {
            host: host.into(),
            port: None,
        },
    );
    manifest
}

#[test]
fn une_cle_de_serveur_fautive_est_signalee() {
    // Sans ce contrôle, une faute de frappe ne provoque rien : la clé ne
    // correspond à aucun environnement, le jeu s'ouvre sur le menu, et
    // cela ressemble exactement à un pack qui n'aurait rien déclaré.
    let problemes = avec_cle("prodution", "mc.ggy.info").problemes_de_serveurs();
    assert_eq!(problemes.len(), 1);
    assert!(problemes[0].contains("prodution"));
}

#[test]
fn un_alias_est_signale_parce_qu_il_ne_serait_pas_lu() {
    // Environment::parse accepte « dev », mais server_for cherche
    // « development » : l'entrée passerait ici et resterait introuvable.
    let problemes = avec_cle("dev", "mc-dev.ggy.info").problemes_de_serveurs();
    assert_eq!(problemes.len(), 1);
    assert!(problemes[0].contains("development"));
}

#[test]
fn une_cle_local_est_signalee() {
    let problemes = avec_cle("local", "mc-dev.ggy.info").problemes_de_serveurs();
    assert_eq!(problemes.len(), 1);
    assert!(problemes[0].contains("development"));
}

#[test]
fn un_hote_vide_est_signale() {
    assert_eq!(
        avec_cle("production", "   ").problemes_de_serveurs().len(),
        1
    );
}

#[test]
fn les_cles_canoniques_ne_posent_aucun_probleme() {
    assert!(avec_serveurs().problemes_de_serveurs().is_empty());
}

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

#[test]
fn un_binaire_local_lit_la_cle_development() {
    assert_eq!(
        Manifest::environnement_serveur(mc_log::Environment::Local),
        mc_log::Environment::Development
    );
    assert_eq!(
        Manifest::environnement_serveur(mc_log::Environment::Production),
        mc_log::Environment::Production
    );
}

#[test]
fn un_manifeste_sans_serveurs_reste_lisible() {
    // Le champ est arrivé après les premiers packs : les manifestes qui
    // l'ignorent doivent continuer de se lire tels quels.
    let brut = br#"{"schema":1,"name":"essai","minecraft":"1.21.1",
                    "loader":{"type":"neoforge","version":"latest"}}"#;
    let manifest = Manifest::parse(brut).expect("manifeste sans serveurs");
    assert!(manifest.servers.is_empty());
    assert!(
        manifest
            .server_for(mc_log::Environment::Production)
            .is_none()
    );
}

#[test]
fn un_manifeste_minimal_est_accepte() {
    assert!(base().check().is_ok());
}

#[test]
fn un_format_inconnu_est_refuse() {
    let mut m = base();
    m.schema = 99;
    assert!(m.check().is_err());
}

#[test]
fn un_mod_en_double_est_refuse() {
    let mut m = base();
    m.mods = vec![
        ModEntry {
            slug: "jei".into(),
            source: None,
            file: None,
            version: None,
            side: None,
            channel: None,
        },
        ModEntry {
            slug: "JEI".into(),
            source: None,
            file: None,
            version: None,
            side: None,
            channel: None,
        },
    ];
    assert!(m.check().is_err());
}

#[test]
fn epingler_deux_fois_la_meme_chose_est_refuse() {
    let mut m = base();
    m.mods = vec![ModEntry {
        slug: "jei".into(),
        source: None,
        file: Some("abcd1234".into()),
        version: Some("19.51.0.418".into()),
        side: None,
        channel: None,
    }];
    assert!(m.check().is_err());
}

#[test]
fn le_java_du_manifeste_prime_sur_celui_de_mojang() {
    let mut m = base();
    assert_eq!(m.java_major(21), 21);
    m.java = Some(22);
    assert_eq!(m.java_major(21), 22);
}

#[test]
fn le_cote_est_lu_depuis_le_texte() {
    let entry = ModEntry {
        slug: "embeddium".into(),
        source: None,
        file: None,
        version: None,
        side: Some("client".into()),
        channel: None,
    };
    assert_eq!(entry.to_request().unwrap().side, Some(Side::Client));
}

#[test]
fn un_cote_inconnu_est_refuse() {
    let entry = ModEntry {
        slug: "x".into(),
        source: None,
        file: None,
        version: None,
        side: Some("les-deux".into()),
        channel: None,
    };
    assert!(entry.to_request().is_err());
}

#[test]
fn aller_retour_json() {
    let json = r#"{
        "schema": 1,
        "name": "samflix",
        "minecraft": "1.21.1",
        "loader": { "type": "neoforge", "version": "latest" },
        "mods": [
            { "slug": "jei" },
            { "slug": "jade", "file": "eYz2YBGT", "source": "modrinth" },
            { "slug": "attributefix", "side": "both", "channel": "beta" }
        ]
    }"#;
    let manifest: Manifest = serde_json::from_str(json).unwrap();
    manifest.check().unwrap();
    assert!(manifest.loader.is_latest());
    assert_eq!(manifest.mods.len(), 3);
    assert_eq!(manifest.mods[1].source, Some(Origin::Modrinth));
    assert_eq!(manifest.mods[2].channel, Some(Channel::Beta));
}
