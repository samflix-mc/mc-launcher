use super::*;

#[test]
fn un_jwt_est_masque_meme_sans_mot_cle() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk";
    let sortie = redact(&format!("échec avec {jwt} en tête"));
    assert!(!sortie.contains("eyJhbGci"));
    assert!(sortie.contains(MASK));
    assert!(sortie.contains("échec avec"));
}

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

#[test]
fn le_nom_du_champ_reste_lisible() {
    // Sans le mot-clé, un incident ne dirait plus de quel jeton il s'agit.
    let sortie = redact("refresh_token=M.C123_BAY");
    assert!(sortie.starts_with("refresh_token="));
    assert!(sortie.ends_with(MASK));
}

#[test]
fn un_texte_sans_secret_est_intact() {
    let texte = "téléchargement de jei-1.21.1-neoforge-19.51.0.418.jar (1.7 Mio)";
    assert_eq!(redact(texte), texte);
}

#[test]
fn une_empreinte_n_est_pas_prise_pour_un_secret() {
    // Les SHA-1 sont utiles au diagnostic et ne révèlent rien.
    let texte = "empreinte 88ee316e68900080b017f60c12162e2731924cf8 attendue";
    assert_eq!(redact(texte), texte);
}

#[test]
fn un_identifiant_court_commencant_par_ey_survit() {
    // « eyZ2YBGT » est un identifiant de version Modrinth, pas un jeton.
    let texte = "build épinglé eyZ2YBGT introuvable";
    assert_eq!(redact(texte), texte);
}

#[test]
fn le_repertoire_personnel_devient_un_tilde() {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return;
    }
    let sortie = redact(&format!("{home}/.local/share/samflix-mc/logs"));
    assert!(sortie.starts_with('~'));
    assert!(!sortie.contains(&home));
}

#[test]
fn plusieurs_secrets_dans_un_meme_texte() {
    let sortie = redact("token=abc123456 puis api_key=def789012 fin");
    assert_eq!(sortie.matches(MASK).count(), 2);
    assert!(sortie.contains("puis"));
    assert!(sortie.ends_with("fin"));
}
