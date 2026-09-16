use crate::redact::redact;

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
