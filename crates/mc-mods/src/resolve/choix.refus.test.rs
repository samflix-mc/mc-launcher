//! Les trois impasses, et ce qu'elles disent au joueur.

use super::{Request, trancher};
use crate::resolve::essais::candidat;

/// Les deux sources ont répondu et aucune ne connaît ce projet. Sans clé, la
/// recherche par mot-clé de CurseForge est fermée : un slug qui ne correspond
/// pas à celui du site n'y est pas trouvable, et c'est la cause la plus
/// fréquente. Le message doit donc envoyer vérifier le slug, pas chercher une
/// panne.
#[test]
fn projet_introuvable_envoie_verifier_le_slug() {
    let erreur = trancher(Vec::new(), &Request::new("jade"), "1.21.1", "neoforge")
        .expect_err("aucun candidat");

    let texte = erreur.to_string();
    assert!(
        texte.contains("introuvable pour Minecraft 1.21.1 / neoforge"),
        "{texte}"
    );
    assert!(texte.contains("slug"), "{texte}");
}

#[test]
fn telechargement_interdit_renvoie_vers_la_page_du_mod() {
    let mut interdit = candidat("jade", "15.10.6");
    interdit.redistributable = false;
    let erreur = trancher(vec![interdit], &Request::new("jade"), "1.21.1", "neoforge")
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
    let erreur = trancher(vec![sans_url], &Request::new("jade"), "1.21.1", "neoforge")
        .expect_err("url vide");
    assert!(erreur.to_string().contains("apports manuels"));
}
