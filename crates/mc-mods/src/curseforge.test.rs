use super::*;

#[test]
fn identifiants_de_chargeur() {
    assert_eq!(loader_type("neoforge"), 6);
    assert_eq!(loader_type("NeoForge"), 6);
    assert_eq!(loader_type("fabric"), 4);
}

#[test]
fn canaux_de_publication() {
    assert_eq!(channel_of(1), Channel::Release);
    assert_eq!(channel_of(2), Channel::Beta);
    assert_eq!(channel_of(3), Channel::Alpha);
}
