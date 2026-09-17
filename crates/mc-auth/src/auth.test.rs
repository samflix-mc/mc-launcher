use super::{Auth, client};

/// Le client n'a qu'un rôle : identifier le launcher et fixer la pile TLS.
/// `minecraft-auth` pose ses propres délais d'expiration par requête.
#[test]
fn le_client_d_authentification_se_construit() {
    assert!(client().is_ok());
}

/// Une session illisible ne doit pas faire paniquer le lancement : elle vaut
/// « reconnecte-toi », et le message doit le dire.
#[test]
fn une_session_illisible_se_dit_au_lieu_de_paniquer() {
    let erreur = echec(&serde_json::json!({"n_importe": "quoi"}));

    assert!(erreur.contains("session enregistrée illisible"), "{erreur}");
}

#[test]
fn un_etat_qui_n_est_pas_un_objet_est_refuse_aussi() {
    echec(&serde_json::Value::Null);
    echec(&serde_json::json!([1, 2, 3]));
}

/// `Auth` n'est pas `Debug` — il détient des jetons, et c'est délibéré :
/// `{:?}` sur cette structure les écrirait dans un journal. D'où cette
/// reprise à la main plutôt qu'un `expect_err`.
fn echec(etat: &serde_json::Value) -> String {
    match Auth::resume(etat) {
        Ok(_) => panic!("ce n'est pas un état de JavaAuthManager"),
        Err(erreur) => format!("{erreur:#}"),
    }
}
