use super::{hors_ligne, logout, whoami};

/// Un répertoire de configuration propre à ce test, et la variable qui y mène.
///
/// SAFETY : le garde pose et retire `XDG_CONFIG_HOME` ; ce binaire ne lance
/// aucun sous-processus, et les tests qui en dépendent sont sérialisés par le
/// verrou ci-dessous.
struct Configuration {
    racine: std::path::PathBuf,
    precedent: Option<std::ffi::OsString>,
}

static OCCUPE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn configuration(nom: &str) -> Configuration {
    use std::sync::atomic::Ordering;
    while OCCUPE
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }

    let racine = std::env::temp_dir().join(format!("mc-auth-cmd-{nom}-{}", std::process::id()));
    std::fs::remove_dir_all(&racine).ok();
    std::fs::create_dir_all(&racine).unwrap();

    let precedent = std::env::var_os("XDG_CONFIG_HOME");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &racine);
    }
    Configuration { racine, precedent }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        unsafe {
            match self.precedent.take() {
                Some(valeur) => std::env::set_var("XDG_CONFIG_HOME", valeur),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
        std::fs::remove_dir_all(&self.racine).ok();
        OCCUPE.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Un profil local ne contacte personne : c'est ce qui permet de jouer sur un
/// serveur en `online-mode=false` sans compte Microsoft.
#[test]
fn le_profil_hors_ligne_ne_contacte_personne() {
    hors_ligne("Sam");
    hors_ligne("thesam1798");
}

/// Se déconnecter sans session enregistrée n'est pas une erreur : c'est l'état
/// dans lequel on veut arriver.
#[test]
fn se_deconnecter_sans_session_reste_sans_effet() {
    let config = configuration("logout");

    logout().expect("aucune session à oublier");
    mc_auth::enregistrer(&serde_json::json!({"refresh_token": "M.R3_BAY.x"})).unwrap();
    assert!(mc_auth::charger().is_some());

    logout().expect("la session est oubliée");
    assert!(mc_auth::charger().is_none());

    drop(config);
}

/// Sans session, `whoami` le dit et rend la main : c'est une question, pas une
/// opération qui peut échouer.
#[tokio::test]
async fn whoami_sans_session_ne_leve_pas_d_erreur() {
    let config = configuration("whoami");

    whoami().await.expect("aucune session n'est pas une erreur");

    drop(config);
}
