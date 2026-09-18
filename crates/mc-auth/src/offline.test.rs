use super::offline_session;

#[test]
fn offline_uuid_conforms_to_the_vanilla_server() {
    // Reference value: UUID.nameUUIDFromBytes("OfflinePlayer:Notch")
    // as computed by a Minecraft server in online-mode=false.
    let s = offline_session("Notch");
    assert_eq!(s.profile.id, "b50ad385829d3141a2167e7d7539ba7f");
    assert_eq!(s.profile.name, "Notch");
    assert!(s.minecraft_token.is_empty(), "no token should be produced");
}
