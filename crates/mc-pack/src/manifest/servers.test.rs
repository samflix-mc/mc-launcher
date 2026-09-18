use super::{Manifest, Server};
use crate::manifest::fixtures::with_servers;

#[test]
fn the_server_follows_the_environment() {
    let manifest = with_servers();
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
        Some("78.46.100.5:25566".into())
    );
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info:25565".into()),
        "a declared port is written, even the default one"
    );
}

#[test]
fn a_local_binary_joins_dev() {
    // A hand-built binary is a working binary. Sending it to production
    // would be like sending someone who's testing to where others play.
    let manifest = with_servers();
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Local)
            .map(Server::address),
        manifest
            .server_for(mc_log::Environment::Development)
            .map(Server::address),
    );
}

#[test]
fn a_preproduction_without_a_server_launches_nothing() {
    // It has no Minecraft servers behind it, and that's not an oversight:
    // the node is unique, every full network costs RAM.
    assert!(
        with_servers()
            .server_for(mc_log::Environment::Preproduction)
            .is_none()
    );
}

#[test]
fn canonical_keys_raise_no_problem() {
    assert!(with_servers().server_problems().is_empty());
}

#[test]
fn a_local_binary_reads_the_development_key() {
    assert_eq!(
        Manifest::server_environment(mc_log::Environment::Local),
        mc_log::Environment::Development
    );
    assert_eq!(
        Manifest::server_environment(mc_log::Environment::Production),
        mc_log::Environment::Production
    );
}

#[test]
fn a_manifest_without_servers_stays_readable() {
    // The field arrived after the first packs: manifests that lack it must
    // keep reading as is.
    let raw = br#"{"schema":1,"name":"test","minecraft":"1.21.1",
                    "loader":{"type":"neoforge","version":"latest"}}"#;
    let manifest = Manifest::parse(raw).expect("manifest without servers");
    assert!(manifest.servers.is_empty());
    assert!(
        manifest
            .server_for(mc_log::Environment::Production)
            .is_none()
    );
}
