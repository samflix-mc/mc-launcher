use super::options;

/// Un jeton assez ressemblant pour que la censure le reconnaisse.
const JETON: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.SflKxwRJSMeKKF2QT4f";

#[test]
fn rien_d_identifiant_ne_part_par_defaut() {
    let options = options();

    // Le processus détient des jetons Microsoft, Xbox Live et Minecraft : la
    // documentation de Sentry propose l'inverse, et c'est précisément ce qu'on
    // ne veut pas.
    assert!(
        !options.send_default_pii,
        "send_default_pii doit rester faux"
    );

    // Sans cette chaîne vide, le SDK renseigne le nom de la machine — sur un
    // poste de joueur, une donnée identifiante qui n'apprend rien sur la panne.
    assert_eq!(options.server_name.as_deref(), Some(""));
}

#[test]
fn la_release_nomme_le_launcher_et_non_le_crate_qui_journalise() {
    // `release_name!()` rendrait « mc-log@… » pour tous les binaires, ce qui
    // interdirait de distinguer une version de mc-pack d'une autre.
    let release = options().release.expect("une release est déclarée");
    assert!(
        release.starts_with("mc-launcher@"),
        "release inattendue : {release}"
    );
}

#[test]
fn l_environnement_est_celui_qui_est_declare() {
    assert_eq!(
        options().environment.as_deref(),
        Some(crate::environment::current().as_str())
    );
}

/// Deux secondes, et pas dix : ce budget est payé par toutes les commandes en
/// se fermant, y compris un `verify` hors ligne qui n'a rien à envoyer.
#[test]
fn l_attente_de_fermeture_reste_courte() {
    assert!(options().shutdown_timeout <= std::time::Duration::from_secs(2));
    // La trace d'appels est jointe même sans exception — c'est elle qui rend
    // un `capture_message` exploitable.
    assert!(options().attach_stacktrace);
}

#[test]
fn un_evenement_est_censure_avant_de_partir() {
    let avant = options()
        .before_send
        .expect("un filtre before_send est posé");

    let event = sentry::protocol::Event {
        message: Some(format!("échec avec access_token={JETON}")),
        ..Default::default()
    };

    let sorti = avant(event).expect("l'événement n'est pas jeté, seulement censuré");
    let message = sorti.message.expect("le message survit");
    assert!(!message.contains("eyJhbGci"), "jeton en clair : {message}");
    assert!(message.contains("échec avec"), "le contexte est perdu");
}

#[test]
fn un_fil_d_ariane_est_censure_lui_aussi() {
    let avant = options()
        .before_breadcrumb
        .expect("un filtre before_breadcrumb est posé");

    let mut crumb = sentry::protocol::Breadcrumb {
        message: Some(format!("requête avec access_token={JETON}")),
        ..Default::default()
    };
    crumb.data.insert(
        "entete".into(),
        sentry::protocol::Value::String(format!("Bearer {JETON}")),
    );

    let sorti = avant(crumb).expect("le fil d'Ariane n'est pas jeté");
    assert!(!sorti.message.unwrap().contains("eyJhbGci"));
    let entete = sorti.data.get("entete").unwrap().as_str().unwrap();
    assert!(!entete.contains("eyJhbGci"), "en-tête en clair : {entete}");
}

/// Les journaux structurés empruntent un canal distinct : `before_send` ne les
/// voit pas. Sans ce second filtre, la censure serait contournée par la voie la
/// plus bavarde de toutes.
#[test]
fn un_journal_structure_est_censure_et_perd_le_nom_de_la_machine() {
    let avant = options()
        .before_send_log
        .expect("un filtre before_send_log est posé");

    let mut attributes = sentry::protocol::Map::new();
    attributes.insert(
        "server.address".into(),
        sentry::protocol::LogAttribute::from("poste-de-sam"),
    );
    attributes.insert(
        "jeton".into(),
        sentry::protocol::LogAttribute::from(JETON.to_string()),
    );
    attributes.insert("essais".into(), sentry::protocol::LogAttribute::from(3));

    let log = sentry::protocol::Log {
        level: sentry::protocol::LogLevel::Info,
        body: format!("échange abouti access_token={JETON}"),
        trace_id: None,
        timestamp: std::time::SystemTime::UNIX_EPOCH,
        severity_number: None,
        attributes,
    };

    let sorti = avant(log).expect("le journal n'est pas jeté");
    assert!(!sorti.body.contains("eyJhbGci"), "corps : {}", sorti.body);
    assert!(
        !sorti.attributes.contains_key("server.address"),
        "le nom de la machine du joueur est parti avec le journal"
    );
    let jeton = &sorti.attributes.get("jeton").unwrap().0;
    assert!(!jeton.as_str().unwrap().contains("eyJhbGci"));
    // Les nombres restent intacts : ils servent au tri et ne portent rien.
    assert_eq!(sorti.attributes.get("essais").unwrap().0, 3);
}
