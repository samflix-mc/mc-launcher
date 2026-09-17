//! Ce que `SAMFLIX_SANS_TROUSSEAU` décide.
//!
//! La variable existe pour deux raisons, et la seconde a coûté une session
//! Microsoft à chaque `cargo test` : le fichier s'isole en déplaçant
//! `XDG_CONFIG_HOME`, le trousseau non — il est unique pour la session de
//! l'utilisateur. Une suite qui lance `mc-auth logout` efface alors la vraie
//! session du poste qui l'exécute.

use super::trousseau_permis;

/// Sérialise les tests qui posent la variable : deux qui la changent en même
/// temps se contredisent.
static VERROU: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Garde(Option<std::ffi::OsString>);

fn poser(valeur: Option<&str>) -> Garde {
    let precedent = std::env::var_os("SAMFLIX_SANS_TROUSSEAU");
    // SAFETY : le verrou garantit qu'aucun autre test de ce binaire ne lit ni
    // n'écrit la variable tant que le garde vit.
    unsafe {
        match valeur {
            Some(valeur) => std::env::set_var("SAMFLIX_SANS_TROUSSEAU", valeur),
            None => std::env::remove_var("SAMFLIX_SANS_TROUSSEAU"),
        }
    }
    Garde(precedent)
}

impl Drop for Garde {
    fn drop(&mut self) {
        unsafe {
            match self.0.take() {
                Some(valeur) => std::env::set_var("SAMFLIX_SANS_TROUSSEAU", valeur),
                None => std::env::remove_var("SAMFLIX_SANS_TROUSSEAU"),
            }
        }
    }
}

#[test]
fn sans_la_variable_le_trousseau_sert() {
    let _verrou = VERROU.lock().unwrap_or_else(|e| e.into_inner());
    let _garde = poser(None);

    assert!(trousseau_permis());
}

#[test]
fn les_trois_formes_d_acceptation_coupent_le_trousseau() {
    let _verrou = VERROU.lock().unwrap_or_else(|e| e.into_inner());

    for valeur in ["1", "true", "oui"] {
        let _garde = poser(Some(valeur));
        assert!(!trousseau_permis(), "« {valeur} » aurait dû couper");
    }
}

/// Une variable posée à autre chose ne coupe rien : « 0 » veut dire « utilise
/// le trousseau », et l'interpréter comme une présence le couperait à
/// l'inverse de ce qui est demandé.
#[test]
fn une_valeur_qui_ne_dit_pas_oui_laisse_le_trousseau() {
    let _verrou = VERROU.lock().unwrap_or_else(|e| e.into_inner());

    for valeur in ["0", "false", "non", ""] {
        let _garde = poser(Some(valeur));
        assert!(trousseau_permis(), "« {valeur} » n'aurait pas dû couper");
    }
}
