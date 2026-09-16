use super::{Request, trancher};
use crate::Channel;
use crate::resolve::essais::candidat;

#[test]
fn le_build_retenu_est_rendu_tel_quel() {
    let retenu = trancher(
        vec![candidat("jade", "15.10.6")],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect("un build retenu");
    assert_eq!(retenu.version_number, "15.10.6");
}

#[test]
fn version_demandee_absente_le_dit_plutot_que_introuvable() {
    let mut request = Request::new("jade");
    request.version = Some("99.0.0".into());
    let erreur = trancher(
        vec![candidat("jade", "15.10.6")],
        &request,
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("version absente");
    let texte = erreur.to_string();
    assert!(texte.contains("aucune version ne correspond"), "{texte}");
    assert!(texte.contains("« 99.0.0 »"), "{texte}");
}

#[test]
fn canal_trop_strict_nomme_le_canal() {
    let mut candidats = vec![candidat("jade", "15.10.6")];
    candidats[0].channel = Channel::Beta;
    let mut request = Request::new("jade");
    request.channel = Some(Channel::Release);
    let erreur =
        trancher(candidats, &request, "1.21.1", "neoforge", true).expect_err("aucune release");
    assert!(erreur.to_string().contains("au canal release"), "{erreur}");
}
