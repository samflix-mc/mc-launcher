use super::assembler;
use crate::essais::{Arbre, NEOFORGE, VANILLA};
use crate::launch::descripteur::resolve_chain;
use crate::launch::session::LaunchOptions;
use crate::launch::variables::active_features;
use crate::vanilla::Features;
use std::collections::BTreeMap;

const OS: &str = "linux";
const ARCH: &str = "x86_64";

/// La chaîne lue depuis un arbre factice : `VersionJson` ne se construit que
/// par désérialisation, et c'est aussi bien — le format lu est celui que
/// Mojang publie.
fn chaine(
    nom: &str,
    descripteurs: &[(&str, &str)],
) -> (Arbre, Vec<crate::launch::descripteur::VersionJson>) {
    let arbre = Arbre::neuf(nom);
    for (id, json) in descripteurs {
        arbre.version(id, json);
    }
    let chaine = resolve_chain(&arbre.shared(), descripteurs[0].0).unwrap();
    (arbre, chaine)
}

fn variables() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("auth_player_name".into(), "Sam".into()),
        ("auth_uuid".into(), "0123".into()),
        ("classpath".into(), "/a.jar".into()),
        ("natives_directory".into(), "/natives".into()),
        ("library_directory".into(), "/libraries".into()),
        ("quickPlayMultiplayer".into(), "mc.ggy.info".into()),
    ])
}

/// Le socle pose ses arguments, le chargeur ajoute les siens par-dessus : la
/// chaîne est parcourue à l'envers de l'héritage.
#[test]
fn le_socle_passe_avant_le_chargeur() {
    let (_arbre, chaine) = chaine(
        "asm-ordre",
        &[("neoforge-21.1.250", NEOFORGE), ("1.21.1", VANILLA)],
    );

    let args = assembler(
        &chaine,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "cpw.mods.bootstraplauncher.BootstrapLauncher".into(),
        &LaunchOptions::default(),
    );

    let jvm_socle = args
        .iter()
        .position(|a| a.starts_with("-Djava.library.path"));
    let jvm_chargeur = args
        .iter()
        .position(|a| a.starts_with("-DlibraryDirectory"));
    assert!(
        jvm_socle < jvm_chargeur,
        "le chargeur passe avant le socle : {args:?}"
    );

    // La classe principale sépare les arguments JVM de ceux du jeu.
    let classe = args
        .iter()
        .position(|a| a == "cpw.mods.bootstraplauncher.BootstrapLauncher")
        .expect("la classe principale est présente");
    assert!(jvm_chargeur.unwrap() < classe);
    assert!(args.iter().position(|a| a == "--launchTarget").unwrap() > classe);
}

/// Un argument conditionnel n'apparaît que si son drapeau est actif. Sans
/// cela, le jeu refuse la ligne de commande.
#[test]
fn les_arguments_conditionnels_suivent_les_drapeaux() {
    let (_arbre, chaine) = chaine("asm-drapeaux", &[("1.21.1", VANILLA)]);
    let options = LaunchOptions {
        quick_play: Some(crate::launch::session::QuickPlay::Multiplayer(
            "mc.ggy.info".into(),
        )),
        ..Default::default()
    };

    let avec = assembler(
        &chaine,
        OS,
        ARCH,
        &active_features(&options),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "M".into(),
        &options,
    );
    assert!(avec.contains(&"--quickPlayMultiplayer".into()), "{avec:?}");
    assert!(avec.contains(&"mc.ggy.info".into()), "{avec:?}");
    // La résolution n'a pas été demandée : ses arguments restent absents.
    assert!(!avec.contains(&"--width".into()), "{avec:?}");

    let sans = assembler(
        &chaine,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "M".into(),
        &LaunchOptions::default(),
    );
    assert!(!sans.contains(&"--quickPlayMultiplayer".into()), "{sans:?}");
}

/// Format d'avant 2017 : le descripteur ne décrit pas les arguments JVM, et
/// ceux du jeu tiennent sur une seule chaîne.
#[test]
fn un_descripteur_d_avant_2017_recoit_un_classpath_pose_a_la_main() {
    let (_arbre, chaine) = chaine(
        "asm-ancien",
        &[(
            "1.21.1",
            r#"{"id":"1.21.1","mainClass":"M","assets":"legacy",
                "minecraftArguments":"--username ${auth_player_name} --uuid ${auth_uuid}"}"#,
        )],
    );

    let args = assembler(
        &chaine,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar:/b.jar",
        "M".into(),
        &LaunchOptions::default(),
    );

    assert_eq!(args[0], "-Djava.library.path=/natives");
    assert_eq!(args[1], "-cp");
    assert_eq!(args[2], "/a.jar:/b.jar");
    assert!(args.contains(&"Sam".into()), "{args:?}");
    assert!(args.contains(&"0123".into()), "{args:?}");
}

/// La mémoire passe avant les arguments du descripteur, pour qu'un réglage
/// explicite puisse être contredit par ce que le chargeur impose s'il y tient.
#[test]
fn la_memoire_et_les_drapeaux_supplementaires_ouvrent_la_ligne() {
    let (_arbre, chaine) = chaine("asm-memoire", &[("1.21.1", VANILLA)]);

    let args = assembler(
        &chaine,
        OS,
        ARCH,
        &Features::new(),
        &variables(),
        std::path::Path::new("/natives"),
        "/a.jar",
        "M".into(),
        &LaunchOptions {
            memory_mb: Some(6144),
            extra_jvm: vec!["-XX:+UseZGC".into()],
            ..Default::default()
        },
    );

    assert_eq!(args[0], "-Xmx6144M");
    assert_eq!(args[1], "-XX:+UseZGC");
}
