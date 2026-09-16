use super::{dsn, flush_incidents, telemetry_active, telemetry_enabled};

#[test]
fn la_telemetrie_se_coupe() {
    // SAFETY : test mono-thread, variable restaurée aussitôt.
    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "0");
    }
    assert!(!telemetry_enabled());
    assert!(dsn().is_none());
    assert!(!telemetry_active());

    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "1");
    }
    assert!(telemetry_enabled());

    unsafe {
        std::env::remove_var("SAMFLIX_TELEMETRY");
    }
    assert!(telemetry_enabled());
}

/// Un opt-out qu'il faut deviner n'en est pas un : les quatre façons d'écrire
/// « non » doivent toutes marcher, en français comme en anglais.
#[test]
fn toutes_les_facons_d_ecrire_non_sont_entendues() {
    for valeur in ["0", "off", "false", "no", "non", " non "] {
        unsafe {
            std::env::set_var("SAMFLIX_TELEMETRY", valeur);
        }
        assert!(!telemetry_enabled(), "« {valeur} » n'a pas coupé");
    }
    unsafe {
        std::env::remove_var("SAMFLIX_TELEMETRY");
    }
}

#[test]
fn un_dsn_vide_desactive_la_remontee() {
    unsafe {
        std::env::set_var("SENTRY_DSN", "   ");
    }
    assert!(dsn().is_none());
    unsafe {
        std::env::remove_var("SENTRY_DSN");
    }
}

/// Un DSN posé remplace celui du projet : c'est ce qui permet de router les
/// incidents d'un déploiement particulier ailleurs.
#[test]
fn un_dsn_pose_remplace_celui_du_projet() {
    unsafe {
        std::env::set_var("SENTRY_DSN", " https://cle@exemple.invalid/7 ");
    }
    assert_eq!(dsn().as_deref(), Some("https://cle@exemple.invalid/7"));
    unsafe {
        std::env::remove_var("SENTRY_DSN");
    }
}

/// Sans client, rien n'a été émis, donc rien ne part : `false` distingue cela
/// d'une file vidée pour de bon.
#[test]
fn sans_client_la_file_n_est_pas_dite_videe() {
    assert!(!flush_incidents(std::time::Duration::from_millis(1)));
}
