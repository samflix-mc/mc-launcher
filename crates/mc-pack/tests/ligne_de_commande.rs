//! Le binaire tel qu'un joueur l'invoque.
//!
//! mc-pack est la commande dont l'existence justifie ce dépôt : c'est elle
//! qu'un joueur lance. Son point d'entrée, son aiguillage et sa page d'aide ne
//! s'atteignent pas depuis le crate — et ce sont pourtant eux qui décident de
//! ce qu'il voit quand il se trompe.
//!
//! Seules les commandes qui ne téléchargent rien sont éprouvées ici : une suite
//! qui installerait un pack de mille mods irait chercher chez Modrinth à chaque
//! exécution.

use std::path::Path;
use std::process::{Command, Output};

fn mc_pack(args: &[&str], maison: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mc-pack"))
        .args(args)
        .env("XDG_DATA_HOME", maison.join("donnees"))
        .env("XDG_CONFIG_HOME", maison.join("config"))
        .env("HOME", maison)
        // Rien ne doit partir chez Sentry parce qu'une suite a tourné.
        .env("SAMFLIX_TELEMETRY", "0")
        .output()
        .expect("le binaire mc-pack a été construit par cargo test")
}

fn atelier(nom: &str) -> std::path::PathBuf {
    let racine = std::env::temp_dir().join(format!(
        "mc-pack-cli-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&racine).ok();
    std::fs::create_dir_all(&racine).unwrap();
    racine
}

/// Sans argument, la page d'aide part sur la sortie d'erreur et le code vaut 2.
///
/// Deux, et non un : c'est la convention qui distingue « je n'ai pas compris ta
/// commande » de « ta commande a échoué ». Un script qui réessaie sur échec ne
/// doit pas réessayer une ligne de commande fautive.
#[test]
fn sans_argument_l_aide_s_affiche_et_le_code_vaut_deux() {
    let maison = atelier("usage");
    let sortie = mc_pack(&[], &maison);

    assert_eq!(sortie.status.code(), Some(2), "code inattendu");

    let aide = String::from_utf8_lossy(&sortie.stderr);
    for commande in ["install", "lock", "verify", "launch", "diagnostic"] {
        assert!(aide.contains(commande), "« {commande} » absent de l'aide");
    }
    // L'aide nomme le pack qu'on installera si l'on ne dit rien : c'est la
    // seule façon de savoir, depuis un poste, quel environnement ce binaire
    // sert.
    assert!(aide.contains("Par défaut"), "{aide}");

    std::fs::remove_dir_all(&maison).ok();
}

/// « diagnostic » est la première chose à demander à quelqu'un dont
/// l'installation échoue : il dit où sont les journaux et si les incidents
/// remontent. Il ne lit aucun manifeste et ne touche à rien — c'est justement
/// ce qu'on lance quand on ne sait pas encore ce qui va de travers.
#[test]
fn le_diagnostic_dit_ou_sont_les_journaux_et_ou_vont_les_incidents() {
    let maison = atelier("diagnostic");
    let sortie = mc_pack(&["diagnostic"], &maison);

    assert!(
        sortie.status.success(),
        "erreurs : {}",
        String::from_utf8_lossy(&sortie.stderr)
    );

    let texte = String::from_utf8_lossy(&sortie.stdout);
    assert!(texte.contains("Journaux"), "{texte}");
    assert!(texte.contains("Remontée d'incidents"), "{texte}");
    // Le répertoire annoncé est bien celui qu'on lui a désigné : un diagnostic
    // qui nomme le mauvais chemin envoie chercher le journal là où il n'est
    // pas.
    assert!(
        texte.contains(maison.join("donnees").to_str().unwrap()),
        "chemin hors du répertoire déclaré : {texte}"
    );
    // La télémétrie est coupée par l'environnement de ce test : le diagnostic
    // doit le dire, sinon il annonce une remontée qui n'aura pas lieu.
    assert!(texte.contains("coupée"), "{texte}");

    std::fs::remove_dir_all(&maison).ok();
}

/// Une commande inconnue n'est pas une commande vide : elle se nomme, pour que
/// la faute de frappe saute aux yeux.
#[test]
fn une_commande_inconnue_est_nommee() {
    let maison = atelier("inconnue");
    let sortie = mc_pack(&["instal"], &maison);

    assert!(!sortie.status.success());
    let erreurs = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreurs.contains("instal"), "{erreurs}");

    std::fs::remove_dir_all(&maison).ok();
}
