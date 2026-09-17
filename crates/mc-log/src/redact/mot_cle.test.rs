use crate::redact::{MASK, redact};

#[test]
fn les_formes_usuelles_de_jeton_sont_couvertes() {
    for entree in [
        "access_token=ya29.A0ARrdaM9xQ",
        "\"refresh_token\": \"M.C123_BAY.0.U.ArFbK\"",
        "Authorization: Bearer abcdef123456",
        "x-api-key: $2a$10$abcdefghijklmnop",
        // UUID inventé : un secret réel n'a rien à faire dans un test,
        // il finirait versionné pour toujours.
        "--api-key 00000000-1111-2222-3333-444444444444",
    ] {
        let sortie = redact(entree);
        assert!(sortie.contains(MASK), "non masqué : {entree} → {sortie}");
    }
}

#[test]
fn un_schema_d_authentification_autre_que_bearer_ne_laisse_pas_passer_la_valeur() {
    // « Basic » porte le couple identifiant/mot de passe en base64 : c'est
    // le contenu le plus sensible que cet en-tête puisse transporter. Le nom
    // du schéma, lui, doit survivre : c'est tout ce qui reste pour savoir de
    // quel en-tête il s'agissait.
    for (entree, attendu) in [
        (
            "Authorization: Basic dXNlcjpwYXNzd29yZA==",
            "Authorization: Basic [secret]",
        ),
        (
            "Authorization: Digest cnonce=abcdef123456",
            "Authorization: Digest [secret]",
        ),
        (
            "authorization: Token abcdef123456",
            "authorization: Token [secret]",
        ),
        // Valeur citée : sans ponctuation admise après le schéma, la ligne
        // ressortait intacte — sans même un masque pour le signaler.
        (
            "Authorization: Basic \"dXNlcjpwYXNzd29yZA==\"",
            "Authorization: Basic \"[secret]\"",
        ),
    ] {
        assert_eq!(redact(entree), attendu, "entrée : {entree}");
    }
}

#[test]
fn un_secret_qui_commence_comme_un_schema_reste_masque_en_entier() {
    // Un schéma n'en est un que s'il forme un mot à part et qu'il introduit
    // quelque chose. Sinon il est la valeur, et la reconnaître déplaçait le
    // masque derrière les premiers caractères du secret.
    for (entree, attendu) in [
        ("token=basicSECRETVALUE", "token=[secret]"),
        ("password=dpop9f3a2b", "password=[secret]"),
        ("secret=token12345", "secret=[secret]"),
        ("api_key=bearerAAAA1111", "api_key=[secret]"),
        ("password: digest", "password: [secret]"),
        ("Authorization: Bearer", "Authorization: [secret]"),
    ] {
        assert_eq!(redact(entree), attendu, "entrée : {entree}");
    }
}

/// Un mot-clé sans valeur derrière lui ne cache rien : poser un masque
/// annoncerait un secret là où il n'y en a pas, et rendrait un journal
/// trompeur — on chercherait une fuite qui n'a pas eu lieu.
#[test]
fn un_mot_cle_qui_n_annonce_rien_ne_se_masque_pas() {
    for texte in [
        "token=",
        "api_key: ",
        "Authorization:",
        "mot de passe oublié, voir secret",
    ] {
        assert_eq!(redact(texte), texte, "entrée : {texte}");
    }
}

/// Le schéma ne s'efface au profit de ce qu'il introduit que s'il introduit
/// quelque chose. Un « Bearer » suivi d'un blanc en fin de ligne n'introduit
/// rien : c'est lui qu'il faut masquer, faute de quoi la ligne ressortirait
/// telle quelle.
#[test]
fn un_schema_suivi_du_vide_reste_la_valeur_a_masquer() {
    assert_eq!(redact("authorization: bearer "), "authorization: [secret] ");
    assert_eq!(
        redact("Authorization: Bearer\n"),
        "Authorization: [secret]\n"
    );
}
