use super::*;
use crate::resolve::essais::installed;

#[test]
fn le_canal_le_plus_recent_est_retenu() {
    let mut vieux = installed("jei", &["jei"], &[]).candidate;
    vieux.published = "2024-01-01".into();
    let mut recent = vieux.clone();
    recent.published = "2025-06-01".into();
    recent.version_number = "19.56".into();

    let choix = pick(vec![vieux, recent], &Request::new("jei")).unwrap();
    assert_eq!(choix.version_number, "19.56");
}

#[test]
fn une_beta_est_ecartee_par_defaut() {
    let mut beta = installed("jei", &["jei"], &[]).candidate;
    beta.channel = Channel::Beta;
    assert!(pick(vec![beta.clone()], &Request::new("jei")).is_none());

    let mut request = Request::new("jei");
    request.channel = Some(Channel::Beta);
    assert!(pick(vec![beta], &request).is_some());
}


#[test]
fn une_version_epinglee_qui_n_existe_pas_ne_retombe_sur_rien() {
    // Silencieusement retomber sur une autre version annulerait tout
    // l'intérêt de l'épinglage.
    let candidate = installed("jei", &["jei"], &[]).candidate;
    let mut request = Request::new("jei");
    request.version = Some("99.99".into());
    assert!(pick(vec![candidate], &request).is_none());
}
