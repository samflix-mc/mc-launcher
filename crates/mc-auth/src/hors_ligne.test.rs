use super::offline_session;

#[test]
fn uuid_hors_ligne_conforme_au_serveur_vanilla() {
    // Valeur de référence : UUID.nameUUIDFromBytes("OfflinePlayer:Notch")
    // tel que le calcule un serveur Minecraft en online-mode=false.
    let s = offline_session("Notch");
    assert_eq!(s.profile.id, "b50ad385829d3141a2167e7d7539ba7f");
    assert_eq!(s.profile.name, "Notch");
    assert!(
        s.minecraft_token.is_empty(),
        "aucun jeton ne doit être produit"
    );
}
