use crate::redact::{redact, MASK};

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
fn plusieurs_secrets_dans_un_meme_texte() {
    let sortie = redact("token=abc123456 puis api_key=def789012 fin");
    assert_eq!(sortie.matches(MASK).count(), 2);
    assert!(sortie.contains("puis"));
    assert!(sortie.ends_with("fin"));
}
