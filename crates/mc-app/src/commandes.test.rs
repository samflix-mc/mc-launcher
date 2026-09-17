//! Ce que les commandes promettent à la fenêtre.
//!
//! Les commandes qui comptent parlent à Microsoft, à Mojang ou au trousseau :
//! ce qui s'en vérifie sans compte, c'est le contrat de sérialisation et les
//! quelques décisions prises en local. Le nom d'un champ est ce que le
//! TypeScript lit, et le renommer casse l'affichage sans casser la compilation
//! d'aucun des deux côtés.

use super::*;
use mc_auth::Profile;

fn session(nom: &str, id: &str) -> Session {
    Session {
        minecraft_token: "jeton".to_string(),
        profile: Profile {
            id: id.to_string(),
            name: nom.to_string(),
        },
    }
}

#[test]
fn le_compte_reprend_le_profil_de_la_session() {
    let compte = Compte::from((&session("Sam", "0123456789abcdef"), true));

    assert_eq!(compte.pseudo, "Sam");
    assert_eq!(compte.uuid, "0123456789abcdef");
    assert!(compte.possede_le_jeu);
}

#[test]
fn l_absence_de_licence_est_transmise_telle_quelle() {
    // Le compte est valide, la connexion a réussi : c'est bien un `false` qui
    // doit arriver à la fenêtre, pas une erreur ni un `true` par défaut. C'est
    // ce qui évite d'installer huit cents mégaoctets pour rien.
    let compte = Compte::from((&session("Sam", "abc"), false));

    assert!(!compte.possede_le_jeu);
}

#[test]
fn le_compte_se_serialise_en_camel_case() {
    let compte = Compte::from((&session("Sam", "abc"), true));

    let json = serde_json::to_value(&compte).expect("sérialisation");
    assert_eq!(json["pseudo"], "Sam");
    assert_eq!(json["uuid"], "abc");
    assert_eq!(json["possedeLeJeu"], true);
}

#[test]
fn le_code_d_appareil_garde_ses_deux_adresses() {
    // La forme longue sert quand le navigateur ne s'ouvre pas ; ne transmettre
    // que la directe laisserait le joueur sans recours.
    let code = CodeAppareil::from(&DeviceCode {
        user_code: "ABCD-EFGH".to_string(),
        verification_uri: "https://microsoft.com/link".to_string(),
        verification_uri_directe: "https://microsoft.com/link?otc=ABCD-EFGH".to_string(),
    });

    assert_eq!(code.code, "ABCD-EFGH");
    assert_eq!(code.url, "https://microsoft.com/link");
    assert_eq!(code.url_directe, "https://microsoft.com/link?otc=ABCD-EFGH");

    let json = serde_json::to_value(&code).expect("sérialisation");
    assert_eq!(
        json["urlDirecte"],
        "https://microsoft.com/link?otc=ABCD-EFGH"
    );
}

#[test]
fn l_erreur_garde_toute_sa_chaine() {
    let erreur = Erreur::from(anyhow::anyhow!("le code a expiré").context("connexion Microsoft"));

    assert_eq!(
        serde_json::to_value(&erreur).expect("sérialisation"),
        serde_json::json!("connexion Microsoft : le code a expiré")
    );
}

#[test]
fn une_erreur_sans_contexte_reste_son_message() {
    let erreur = Erreur::from(anyhow::anyhow!("trousseau verrouillé"));

    assert_eq!(erreur, Erreur("trousseau verrouillé".to_string()));
}

#[test]
fn le_chemin_est_complet_et_ordonne() {
    // La fenêtre le demande une fois, à l'ouverture, pour dessiner ce qui
    // reste à faire. Un chemin partiel ne montrerait jamais la fin.
    let chemin = chemin();

    assert_eq!(chemin.len(), Phase::TOUTES.len());
    let rangs: Vec<usize> = chemin.iter().map(|etape| etape.rang).collect();
    assert_eq!(rangs, (0..Phase::TOUTES.len()).collect::<Vec<_>>());
    assert!(chemin.iter().all(|etape| !etape.libelle.is_empty()));
}

#[test]
fn le_chemin_se_serialise_avec_ses_libelles() {
    let json = serde_json::to_value(chemin()).expect("sérialisation");

    assert_eq!(json[0]["phase"], "connexion");
    assert_eq!(json[0]["libelle"], "Compte Microsoft");
}

#[test]
fn fermer_le_jeu_n_est_pas_une_panne() {
    // Un joueur qui quitte sa partie ne doit pas voir un bandeau rouge. Seul
    // un code de sortie non nul en est un.
    let rapport = mc_instance::launch::Report {
        outcome: mc_instance::launch::Outcome::Normal,
        errors: Vec::new(),
    };
    assert_eq!(verdict(&rapport), "Partie terminée.");

    let interrompu = mc_instance::launch::Report {
        outcome: mc_instance::launch::Outcome::Interrupted { signal: 15 },
        errors: Vec::new(),
    };
    assert_eq!(verdict(&interrompu), "Jeu fermé.");
}

#[test]
fn un_plantage_est_nomme_avec_son_code() {
    let rapport = mc_instance::launch::Report {
        outcome: mc_instance::launch::Outcome::Failed { code: 1 },
        errors: Vec::new(),
    };

    let dit = verdict(&rapport);
    assert!(dit.contains("erreur"), "{dit}");
    assert!(dit.contains('1'), "{dit}");
}

#[test]
fn le_nom_de_l_evenement_ne_bouge_pas() {
    // Le TypeScript écoute cette chaîne-là. Un renommage d'un seul côté laisse
    // une fenêtre qui attend indéfiniment un code déjà émis.
    assert_eq!(EVENEMENT_CODE, "auth://code");
}
