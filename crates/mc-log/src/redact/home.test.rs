use super::maison_utilisable;
use crate::redact::redact;

/// Deux valeurs doivent être refusées, et pour des raisons opposées. Une
/// variable vide ne désigne rien — remplacer la chaîne vide par « ~ » insérerait
/// un tilde entre chaque caractère du journal. Et « / » préfixe tout : un
/// journal entier deviendrait illisible, chemins système compris.
#[test]
fn une_maison_vide_ou_reduite_a_la_racine_ne_sert_pas_de_remplacement() {
    assert_eq!(maison_utilisable("/home/sam").as_deref(), Some("/home/sam"));
    assert_eq!(maison_utilisable(""), None);
    assert_eq!(maison_utilisable("/"), None);
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
