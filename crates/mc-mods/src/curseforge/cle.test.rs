use super::{KEY_REFUSED, api_key, config_key_path, is_key_error};

/// Le fichier permet de ne pas exporter la clé dans chaque shell, et de ne pas
/// la voir passer dans l'historique des commandes. Il vit dans la
/// configuration, à côté de la session.
#[test]
fn la_cle_vit_dans_la_configuration() {
    assert!(
        config_key_path().ends_with("samflix-mc/curseforge.key"),
        "{:?}",
        config_key_path()
    );
}

/// L'environnement prime sur le fichier, et une valeur vide n'est pas une clé :
/// une variable exportée à blanc masquerait le fichier sans rien apporter.
#[test]
fn l_environnement_prime_et_une_valeur_vide_ne_compte_pas() {
    // Le garde sérialise avec l'autre test qui pose ces variables, et rend
    // leur valeur d'origine.
    let _garde = garde_environnement();

    unsafe {
        std::env::set_var("CURSEFORGE_API_KEY", "  $2a$10$depuis-l-environnement  ");
    }
    assert_eq!(api_key().as_deref(), Some("$2a$10$depuis-l-environnement"));

    unsafe {
        std::env::set_var("CURSEFORGE_API_KEY", "   ");
    }
    // Retombe sur le fichier, qui n'existe probablement pas sur ce poste ;
    // dans les deux cas ce n'est pas la valeur vide qui est rendue.
    assert_ne!(api_key().as_deref(), Some("   "));
}

/// Le registre distingue « cette clé ne vaut rien » — auquel cas il bascule sur
/// l'accès sans clé — d'une panne de réseau, qui doit remonter.
#[test]
fn seul_le_refus_de_cle_est_reconnu_comme_tel() {
    let refus = anyhow::anyhow!("{KEY_REFUSED} (HTTP 403)");
    assert!(is_key_error(&refus));

    // Y compris enfoui sous plusieurs couches de contexte, ce qui est le cas
    // réel : l'erreur traverse la requête, puis la résolution.
    let enfoui = refus.context("candidats pour jei").context("résolution");
    assert!(is_key_error(&enfoui));

    assert!(!is_key_error(&anyhow::anyhow!("HTTP 500 : passerelle")));
    assert!(!is_key_error(&anyhow::anyhow!("délai dépassé")));
}

/// À défaut de variable, la clé se lit dans la configuration — c'est la façon
/// dont elle est posée sur un poste de joueur, une fois pour toutes. Un fichier
/// réduit à des blancs ne vaut pas une clé : l'envoyer ferait refuser chaque
/// requête par CurseForge, et le registre basculerait sur l'accès sans clé pour
/// une raison qu'il ne saurait pas nommer.
#[test]
fn a_defaut_de_variable_la_cle_se_lit_dans_le_fichier() {
    let _garde = garde_environnement();
    let racine = std::env::temp_dir().join(format!(
        "mc-mods-cle-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&racine).ok();
    let dossier = racine.join("samflix-mc");
    std::fs::create_dir_all(&dossier).unwrap();

    // SAFETY : le garde sérialise les tests de ce fichier, seuls à lire ces
    // deux variables dans tout le crate, et les restaure en se détruisant.
    unsafe {
        std::env::remove_var("CURSEFORGE_API_KEY");
        std::env::set_var("XDG_CONFIG_HOME", &racine);
    }

    std::fs::write(
        dossier.join("curseforge.key"),
        "  $2a$10$depuis-le-fichier\n",
    )
    .unwrap();
    assert_eq!(api_key().as_deref(), Some("$2a$10$depuis-le-fichier"));

    std::fs::write(dossier.join("curseforge.key"), "   \n").unwrap();
    assert_eq!(api_key(), None, "un fichier de blancs n'est pas une clé");

    std::fs::remove_file(dossier.join("curseforge.key")).unwrap();
    assert_eq!(api_key(), None, "aucun fichier, aucune clé");

    std::fs::remove_dir_all(&racine).ok();
}

/// Sérialise les tests qui posent les variables de ce module, et rend leur
/// valeur d'origine. Sans cela, deux d'entre eux se contredisent — et depuis
/// l'édition 2024, un `set_var` concurrent n'est pas une course mais un
/// comportement indéfini.
fn garde_environnement() -> GardeEnvironnement {
    use std::sync::atomic::Ordering;
    while OCCUPE
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    GardeEnvironnement {
        cle: std::env::var_os("CURSEFORGE_API_KEY"),
        config: std::env::var_os("XDG_CONFIG_HOME"),
    }
}

static OCCUPE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct GardeEnvironnement {
    cle: Option<std::ffi::OsString>,
    config: Option<std::ffi::OsString>,
}

impl Drop for GardeEnvironnement {
    fn drop(&mut self) {
        // SAFETY : le verrou n'est rendu qu'après cette restitution.
        unsafe {
            match &self.cle {
                Some(valeur) => std::env::set_var("CURSEFORGE_API_KEY", valeur),
                None => std::env::remove_var("CURSEFORGE_API_KEY"),
            }
            match &self.config {
                Some(valeur) => std::env::set_var("XDG_CONFIG_HOME", valeur),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
        OCCUPE.store(false, std::sync::atomic::Ordering::Release);
    }
}
