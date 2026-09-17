//! Ce qui se vérifie sans trousseau.
//!
//! Écrire dans le Secret Service depuis une suite de tests demanderait un bus
//! de session et un portefeuille déverrouillé — c'est-à-dire de tester
//! l'environnement plutôt que le code. Ce qui reste, et qui décide de tout le
//! reste du module, c'est l'aiguillage : quelle erreur vaut « rien
//! d'enregistré » et quelle erreur est une panne.

use super::{ENTREE, SERVICE, absente};
use keyring::Error;

#[test]
fn absence_reconnue() {
    assert!(absente(&Error::NoEntry));
}

#[test]
fn panne_non_confondue_avec_une_absence() {
    // Un trousseau verrouillé, une plateforme sans magasin, une entrée
    // ambiguë : aucune ne veut dire que le joueur n'est pas connecté, et les
    // traiter comme telles referait silencieusement passer la session au
    // fichier.
    assert!(!absente(&Error::NoDefaultStore));
    assert!(!absente(&Error::BadEncoding(vec![0xff])));
    assert!(!absente(&Error::Invalid(
        "service".to_string(),
        "vide".to_string()
    )));
}

#[test]
fn le_service_et_l_entree_ne_bougent_pas() {
    // Renommer l'un ou l'autre rend invisible la session déjà enregistrée : le
    // joueur se retrouve déconnecté sans explication, et l'ancienne entrée
    // reste dans son trousseau avec un jeton valide dedans.
    assert_eq!(SERVICE, "samflix-mc");
    assert_eq!(ENTREE, "session-minecraft");
}

/// Le trousseau de cette machine garde-t-il vraiment ce qu'on lui confie ?
///
/// Ignoré par défaut : il écrit dans le portefeuille de l'utilisateur, ce
/// qu'une suite ne doit pas faire sans qu'on le demande. À lancer quand une
/// session s'évapore d'un lancement à l'autre :
///
/// ```sh
/// cargo test -p mc-auth -- --ignored --nocapture le_trousseau_garde
/// ```
///
/// Écrit sous un service distinct de celui du launcher, pour ne pas écraser la
/// session en cours, et nettoie derrière lui.
#[test]
#[ignore = "touche au portefeuille de la machine"]
fn le_trousseau_garde_ce_qu_on_lui_confie() {
    const ESSAI: &str = "samflix-mc-essai";

    let entree = keyring::Entry::new(ESSAI, "diagnostic").expect("le trousseau s'ouvre");
    entree.set_password("valeur-temoin").expect("écriture");

    // Relu par une entrée neuve : la première peut très bien avoir gardé la
    // valeur en mémoire sans que rien ne soit persisté.
    let relu = keyring::Entry::new(ESSAI, "diagnostic")
        .expect("le trousseau s'ouvre")
        .get_password();

    // `MC_TROUSSEAU_TEMOIN=1` laisse l'entrée en place : c'est ce qui permet
    // de vérifier depuis un autre processus — `secret-tool lookup service
    // samflix-mc-essai username diagnostic` — qu'elle a bien été persistée et
    // pas seulement gardée en mémoire par le portefeuille.
    if std::env::var_os("MC_TROUSSEAU_TEMOIN").is_none() {
        let _ = entree.delete_credential();
    }

    assert_eq!(
        relu.as_deref().ok(),
        Some("valeur-temoin"),
        "le trousseau a accepté l'écriture sans la rendre : {relu:?}"
    );
}
