use super::{Channel, channel_of, loader_type};

#[test]
fn identifiants_de_chargeur() {
    // Ces nombres sont ceux de l'API CurseForge : s'en écarter ne produit
    // aucune erreur, seulement une liste de fichiers pour un autre chargeur —
    // et un pack NeoForge rempli de jars Forge.
    assert_eq!(loader_type("neoforge"), 6);
    assert_eq!(loader_type("NeoForge"), 6);
    assert_eq!(loader_type("forge"), 1);
    assert_eq!(loader_type("fabric"), 4);
    assert_eq!(loader_type("quilt"), 5);
    // Un chargeur inconnu vaut NeoForge : c'est celui du réseau, et refuser
    // laisserait le pack sans candidat plutôt qu'avec un mauvais.
    assert_eq!(loader_type("rift"), 6);
}

#[test]
fn canaux_de_publication() {
    assert_eq!(channel_of(1), Channel::Release);
    assert_eq!(channel_of(2), Channel::Beta);
    assert_eq!(channel_of(3), Channel::Alpha);
}
