use super::build;
use crate::essais::{Arbre, NEOFORGE, VANILLA};
use crate::launch::session::{LaunchOptions, QuickPlay, Session};

/// Une installation complète, chargeur compris, prête à être lancée.
fn installation(nom: &str) -> Arbre {
    let arbre = Arbre::neuf(nom);
    arbre
        .version("1.21.1", VANILLA)
        .client("1.21.1")
        .version("neoforge-21.1.250", NEOFORGE)
        // Celle du chargeur l'emporte sur celle du jeu : c'est la seule des
        // deux qui doit être présente.
        .bibliotheque("com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar")
        .bibliotheque("net/neoforged/fancymodloader/loader/4.0.24/loader-4.0.24.jar");
    arbre
}

fn session() -> Session {
    Session::offline("Sam", "0123456789abcdef0123456789abcdef")
}

#[test]
fn la_ligne_de_commande_se_compose_du_socle_et_du_chargeur() {
    let arbre = installation("build");
    let commande = build(
        "neoforge-21.1.250",
        &arbre.shared(),
        &arbre.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect("la ligne de commande s'assemble");

    // La classe principale est celle du chargeur, pas celle du jeu : c'est le
    // premier de la chaîne qui en déclare une qui gagne.
    assert!(
        commande
            .args
            .contains(&"cpw.mods.bootstraplauncher.BootstrapLauncher".into()),
        "{:?}",
        commande.args
    );
    // L'index d'assets vient du socle, que le chargeur ne déclare pas.
    assert!(commande.args.contains(&"--launchTarget".into()));
    assert_eq!(commande.java, std::path::Path::new("/usr/bin/java"));
    assert_eq!(commande.working_dir, arbre.game_dir());

    // Les arguments du socle passent avant ceux du chargeur, et les `${…}`
    // sont résolus.
    assert!(commande.args.contains(&"Sam".into()));
    assert!(!commande.args.iter().any(|a| a.contains("${")));
}

/// LWJGL sort lui-même ses binaires des jars du classpath, mais refuse de
/// démarrer si le répertoire n'existe pas. Le répertoire de jeu, lui, est celui
/// où Minecraft écrira `saves` et `options.txt`.
#[test]
fn les_repertoires_que_la_jvm_exige_sont_crees() {
    let arbre = installation("repertoires");
    build(
        "neoforge-21.1.250",
        &arbre.shared(),
        &arbre.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .unwrap();

    assert!(arbre.shared().join("natives").join("1.21.1").is_dir());
    assert!(arbre.game_dir().is_dir());
}

/// `--quickPlayMultiplayer` n'existe dans le descripteur que derrière une règle
/// conditionnée au drapeau. Ignorer les règles de drapeaux produit une ligne de
/// commande que le jeu refuse.
#[test]
fn rejoindre_un_serveur_ajoute_les_arguments_qui_vont_avec() {
    let arbre = installation("quickplay");
    let commande = build(
        "neoforge-21.1.250",
        &arbre.shared(),
        &arbre.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions {
            quick_play: Some(QuickPlay::Multiplayer("mc.ggy.info".into())),
            memory_mb: Some(4096),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(commande.args.contains(&"--quickPlayMultiplayer".into()));
    assert!(commande.args.contains(&"mc.ggy.info".into()));
    // La mémoire passe avant les arguments du descripteur, pour qu'un réglage
    // explicite puisse être contredit par ce que le chargeur impose.
    assert_eq!(commande.args.first().map(String::as_str), Some("-Xmx4096M"));
    // Sans demande de résolution, ses arguments restent absents.
    assert!(!commande.args.contains(&"--width".into()));
}

#[test]
fn une_version_absente_se_dit_au_lieu_de_planter() {
    let arbre = Arbre::neuf("absente");
    let erreur = build(
        "neoforge-21.1.250",
        &arbre.shared(),
        &arbre.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect_err("rien n'est installé");

    assert!(
        format!("{erreur:#}").contains("n'est pas installé"),
        "{erreur:#}"
    );
}

/// Un descripteur sans classe principale ne peut rien lancer, et le dire vaut
/// mieux que de laisser la JVM répondre à notre place.
#[test]
fn une_chaine_sans_classe_principale_est_refusee() {
    let arbre = Arbre::neuf("sans-classe");
    arbre
        .version("1.21.1", r#"{"id":"1.21.1","assetIndex":{"id":"17"}}"#)
        .client("1.21.1");

    let erreur = build(
        "1.21.1",
        &arbre.shared(),
        &arbre.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect_err("aucune classe principale");

    assert!(
        format!("{erreur:#}").contains("classe principale"),
        "{erreur:#}"
    );
}

/// Les très vieilles versions nomment leur index d'assets par `assets` et non
/// par `assetIndex` ; les deux doivent être lus.
#[test]
fn un_index_d_assets_a_l_ancienne_est_accepte() {
    let arbre = Arbre::neuf("assets-ancien");
    arbre
        .version(
            "1.21.1",
            r#"{"id":"1.21.1","mainClass":"M","assets":"legacy",
                "minecraftArguments":"--username ${auth_player_name}"}"#,
        )
        .client("1.21.1");

    let commande = build(
        "1.21.1",
        &arbre.shared(),
        &arbre.game_dir(),
        std::path::Path::new("/usr/bin/java"),
        &session(),
        &LaunchOptions::default(),
    )
    .expect("un descripteur d'avant 2017 reste lançable");

    // Sans arguments JVM déclarés, le classpath est posé à la main.
    assert!(commande.args.contains(&"-cp".into()));
    assert!(commande.args.contains(&"Sam".into()));
}
