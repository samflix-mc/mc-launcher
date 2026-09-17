use super::{dsn, flush_incidents, telemetry_active, telemetry_enabled};

#[test]
fn la_telemetrie_se_coupe() {
    let vars = crate::essais::variables();

    vars.poser("SAMFLIX_TELEMETRY", "0");
    assert!(!telemetry_enabled());
    assert!(dsn().is_none());
    assert!(!telemetry_active());

    vars.poser("SAMFLIX_TELEMETRY", "1");
    assert!(telemetry_enabled());

    // L'opt-out se déclare ; son absence ne coupe rien.
    vars.retirer("SAMFLIX_TELEMETRY");
    assert!(telemetry_enabled());
}

/// Un opt-out qu'il faut deviner n'en est pas un : les quatre façons d'écrire
/// « non » doivent toutes marcher, en français comme en anglais.
#[test]
fn toutes_les_facons_d_ecrire_non_sont_entendues() {
    let vars = crate::essais::variables();
    for valeur in ["0", "off", "false", "no", "non", " non "] {
        vars.poser("SAMFLIX_TELEMETRY", valeur);
        assert!(!telemetry_enabled(), "« {valeur} » n'a pas coupé");
    }
}

#[test]
fn un_dsn_vide_desactive_la_remontee() {
    let vars = crate::essais::variables();
    // La télémétrie doit être active, sans quoi le DSN ne serait pas même
    // consulté et le test passerait pour la mauvaise raison.
    vars.retirer("SAMFLIX_TELEMETRY");
    vars.poser("SENTRY_DSN", "   ");
    assert!(dsn().is_none());
}

/// Un DSN posé remplace celui du projet : c'est ce qui permet de router les
/// incidents d'un déploiement particulier ailleurs.
#[test]
fn un_dsn_pose_remplace_celui_du_projet() {
    let vars = crate::essais::variables();
    vars.retirer("SAMFLIX_TELEMETRY");
    vars.poser("SENTRY_DSN", " https://cle@exemple.invalid/7 ");
    assert_eq!(dsn().as_deref(), Some("https://cle@exemple.invalid/7"));
}

/// Sans client, rien n'a été émis, donc rien ne part : `false` distingue cela
/// d'une file vidée pour de bon.
#[test]
fn sans_client_la_file_n_est_pas_dite_videe() {
    assert!(!flush_incidents(std::time::Duration::from_millis(1)));
}

/// Et avec un client, la file part et le dit. Les deux appelants qui attendent
/// ce `true` annoncent un identifiant au joueur : « consigné » ne se dit pas
/// comme « transmis », et chercher dans le tableau de bord un identifiant qui
/// n'y est jamais arrivé coûte plus cher que l'attente qu'on s'épargnait.
#[test]
fn avec_un_client_la_file_part_et_le_dit() {
    sentry::test::with_captured_events(|| {
        assert!(
            flush_incidents(std::time::Duration::from_secs(1)),
            "un client est lié : la file part"
        );
    });
}

/// Sans opt-out ni DSN de remplacement, c'est le projet du launcher qui reçoit
/// — et la remontée est donc active. Répondre `false` couperait en silence la
/// seule voie par laquelle un plantage chez un joueur nous parvient.
#[test]
fn sans_rien_declarer_la_remontee_est_active() {
    let vars = crate::essais::variables();
    vars.retirer("SAMFLIX_TELEMETRY");
    vars.retirer("SENTRY_DSN");

    assert!(telemetry_enabled());
    assert!(telemetry_active());
    assert!(dsn().is_some());
}
