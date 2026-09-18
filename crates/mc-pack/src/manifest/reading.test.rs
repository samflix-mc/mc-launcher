use super::Manifest;
use crate::manifest::Server;
use crate::manifest::fixtures::with_servers;

#[test]
fn a_pack_newer_than_the_binary_stays_installable() {
    // The day mc-content declares one more environment, binaries already
    // out with players won't know it. If they rejected the manifest for
    // that, a line added to the pack would cut off installation for the
    // entire install base at once — and nobody could download anything to
    // fix themselves.
    let raw = br#"{"schema":1,"name":"test","minecraft":"1.21.1",
                    "loader":{"type":"neoforge","version":"latest"},
                    "servers":{"production":{"host":"mc.ggy.info"},
                               "qualification":{"host":"mc-qa.ggy.info"}},
                    "unknown_new_field":true}"#;
    let manifest = Manifest::parse(raw).expect("an unknown environment isn't a fault");
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info".into()),
        "what this binary knows how to read stays readable"
    );
    assert!(
        manifest
            .server_for(mc_log::Environment::Development)
            .is_none()
    );
}

#[test]
fn servers_survive_a_pass_through_the_cache() {
    // launch reads the manifest stored in the cache, not the one from the
    // network: a field lost on write would make the game open on the menu
    // instead of joining the server, and only while offline.
    let folder = std::env::temp_dir().join(format!("mc-pack-test-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("samflix.json");
    with_servers().save(&path).unwrap();
    let reread = Manifest::load(&path).unwrap();
    std::fs::remove_dir_all(&folder).ok();
    assert_eq!(
        reread
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
        Some("78.46.100.5:25566".into())
    );
}
