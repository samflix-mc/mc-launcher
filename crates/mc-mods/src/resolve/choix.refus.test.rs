//! Les trois impasses, et ce qu'elles disent au joueur.

use super::*;
use crate::resolve::essais::candidat;

#[test]
fn projet_introuvable_signale_la_source_non_consultee() {
    let erreur = trancher(
        Vec::new(),
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        false,
    )
    .expect_err("aucun candidat");
    let texte = erreur.to_string();
    assert!(
        texte.contains("introuvable pour Minecraft 1.21.1 / neoforge"),
        "{texte}"
    );
    assert!(
        texte.contains("aucune clé CurseForge configurée"),
        "{texte}"
    );

    let avec_cle = trancher(
        Vec::new(),
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("aucun candidat");
    assert!(!avec_cle.to_string().contains("CurseForge configurée"));
}

#[test]
fn telechargement_interdit_renvoie_vers_la_page_du_mod() {
    let mut interdit = candidat("jade", "15.10.6");
    interdit.redistributable = false;
    let erreur = trancher(
        vec![interdit],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("non redistribuable");
    let texte = erreur.to_string();
    assert!(texte.contains("apports manuels"), "{texte}");
    assert!(texte.contains("https://modrinth.com/mod/jade"), "{texte}");
}

/// Une URL vide vaut un téléchargement impossible : la distinction ne se
/// voit qu'au moment de télécharger, trop tard pour l'expliquer.
#[test]
fn url_vide_vaut_telechargement_interdit() {
    let mut sans_url = candidat("jade", "15.10.6");
    sans_url.url = String::new();
    let erreur = trancher(
        vec![sans_url],
        &Request::new("jade"),
        "1.21.1",
        "neoforge",
        true,
    )
    .expect_err("url vide");
    assert!(erreur.to_string().contains("apports manuels"));
}
