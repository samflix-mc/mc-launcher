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
