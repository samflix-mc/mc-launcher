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
    // SAFETY : la variable est restaurée avant la fin du test, et ce crate ne
    // lance aucun sous-processus.
    let precedent = std::env::var_os("CURSEFORGE_API_KEY");

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

    unsafe {
        match precedent {
            Some(valeur) => std::env::set_var("CURSEFORGE_API_KEY", valeur),
            None => std::env::remove_var("CURSEFORGE_API_KEY"),
        }
    }
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
