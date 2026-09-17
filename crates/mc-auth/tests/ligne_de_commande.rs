//! Le binaire tel qu'un joueur l'invoque.
//!
//! Tout ce qui précède est vérifié fonction par fonction, dans le crate.
//! Reste ce qu'aucun appel direct n'atteint : le point d'entrée, l'aiguillage
//! vers la commande, le code de sortie rendu au shell, et ce qui s'imprime.
//! Ces quatre-là ne se voient qu'en lançant l'exécutable — une fonction qui
//! n'imprime plus rien ne fait tomber aucun test qui se contente de l'appeler.

use std::path::Path;
use std::process::{Command, Output};

/// Lance le binaire dans un répertoire personnel à lui.
///
/// `mc_log::init` ouvre un journal dans le répertoire de données dès la
/// première ligne de `main` : sans ces variables, chaque exécution écrirait
/// dans celui du poste qui lance la suite.
fn mc_auth(args: &[&str], maison: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mc-auth"))
        .args(args)
        .env("XDG_DATA_HOME", maison.join("donnees"))
        .env("XDG_CONFIG_HOME", maison.join("config"))
        .env("HOME", maison)
        // Rien ne doit partir chez Sentry parce qu'une suite a tourné.
        .env("SAMFLIX_TELEMETRY", "0")
        // Et surtout : pas de trousseau. Le fichier de session s'isole en
        // déplaçant XDG_CONFIG_HOME, le trousseau non — il est unique pour la
        // session de l'utilisateur. Sans cette variable, « logout » efface la
        // vraie session Microsoft du poste qui exécute la suite, et le joueur
        // doit se reconnecter après chaque « cargo test ».
        .env("SAMFLIX_SANS_TROUSSEAU", "1")
        .output()
        .expect("le binaire mc-auth a été construit par cargo test")
}

fn atelier(nom: &str) -> std::path::PathBuf {
    let racine = std::env::temp_dir().join(format!(
        "mc-auth-cli-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&racine).ok();
    std::fs::create_dir_all(&racine).unwrap();
    racine
}

/// Le profil hors-ligne est la seule commande qui aboutisse sans Microsoft :
/// c'est donc la seule qui vérifie, de bout en bout, qu'un pseudo entre d'un
/// côté et qu'une identité sort de l'autre.
#[test]
fn le_profil_hors_ligne_affiche_le_pseudo_et_son_uuid() {
    let maison = atelier("hors-ligne");
    let sortie = mc_auth(&["--offline", "Notch"], &maison);

    assert!(
        sortie.status.success(),
        "code {:?}, erreurs : {}",
        sortie.status.code(),
        String::from_utf8_lossy(&sortie.stderr)
    );

    let texte = String::from_utf8_lossy(&sortie.stdout);
    assert!(texte.contains("Notch"), "{texte}");
    // L'UUID que calcule un serveur en online-mode=false pour ce pseudo. Le
    // voir ici prouve que l'affichage rend bien ce que le calcul a produit.
    assert!(
        texte.contains("b50ad385829d3141a2167e7d7539ba7f"),
        "{texte}"
    );
    // Et qu'il dit ce que cette session ne permet pas.
    assert!(texte.contains("online-mode=false"), "{texte}");

    std::fs::remove_dir_all(&maison).ok();
}

/// Sans commande, le binaire doit rappeler ce qu'il attend *et* sortir en
/// erreur. Un code de sortie nul ferait croire à un script appelant que la
/// session est ouverte.
#[test]
fn sans_commande_l_usage_est_rappele_et_le_code_est_non_nul() {
    let maison = atelier("sans-commande");
    let sortie = mc_auth(&[], &maison);

    assert!(
        !sortie.status.success(),
        "une invocation vide ne peut pas réussir"
    );

    let erreurs = String::from_utf8_lossy(&sortie.stderr);
    assert!(erreurs.contains("mc-auth login"), "{erreurs}");
    assert!(erreurs.contains("--offline <PSEUDO>"), "{erreurs}");

    std::fs::remove_dir_all(&maison).ok();
}

/// `logout` sans session n'est pas une erreur : c'est l'état où l'on voulait
/// arriver. Le code de sortie doit le dire.
#[test]
fn oublier_une_session_absente_reussit() {
    let maison = atelier("logout");
    let sortie = mc_auth(&["logout"], &maison);

    assert!(
        sortie.status.success(),
        "erreurs : {}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    let texte = String::from_utf8_lossy(&sortie.stdout);
    assert!(texte.contains("oubliée"), "{texte}");

    std::fs::remove_dir_all(&maison).ok();
}

/// `whoami` sans session enregistrée renvoie vers `login` plutôt que de
/// tomber : c'est le cas d'un premier lancement, pas une panne.
#[test]
fn se_nommer_sans_session_renvoie_vers_login() {
    let maison = atelier("whoami");
    let sortie = mc_auth(&["whoami"], &maison);

    assert!(
        sortie.status.success(),
        "erreurs : {}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    let texte = String::from_utf8_lossy(&sortie.stdout);
    assert!(texte.contains("mc-auth login"), "{texte}");

    std::fs::remove_dir_all(&maison).ok();
}
