use super::{Server, already_has_a_port};
use crate::manifest::fixtures::{base, server};

#[test]
fn an_ipv6_address_gets_bracketed() {
    // Guava, which Minecraft uses to read the address, rejects anything
    // carrying more than one “:” without brackets. A bare IPv6 address
    // therefore didn't give a wrong address: it gave none at all, and the
    // game opened on the menu as if the pack had declared nothing.
    assert_eq!(
        server("2001:db8::1", Some(25566)).address(),
        "[2001:db8::1]:25566"
    );
    assert_eq!(server("2001:db8::1", None).address(), "[2001:db8::1]");
    assert_eq!(
        server("[2001:db8::1]", Some(25566)).address(),
        "[2001:db8::1]:25566",
        "brackets already in place aren't doubled"
    );
}

#[test]
fn a_hostname_and_an_ipv4_address_stay_intact() {
    assert_eq!(server("mc.ggy.info", None).address(), "mc.ggy.info");
    assert_eq!(
        server("78.46.100.5", Some(25566)).address(),
        "78.46.100.5:25566"
    );
}

#[test]
fn a_host_already_suffixed_with_a_port_is_flagged() {
    // “mc.ggy.info:25566” plus a “port” field would compose
    // “mc.ggy.info:25566:25570”, which Minecraft rejects — and the
    // rejection is silent, just like a bare IPv6 address.
    let mut manifest = base();
    manifest.servers.insert(
        "production".into(),
        server("mc.ggy.info:25566", Some(25570)),
    );
    assert_eq!(manifest.server_problems().len(), 1);

    // Only the duplicate is a problem: a suffixed host without a “port”
    // field composes a valid address, and nothing justifies rejecting it.
    let mut manifest = base();
    manifest
        .servers
        .insert("production".into(), server("mc.ggy.info:25566", None));
    assert!(manifest.server_problems().is_empty());
    assert_eq!(
        manifest
            .server_for(mc_log::Environment::Production)
            .map(Server::address),
        Some("mc.ggy.info:25566".into())
    );
}

/// A host that already carries its port must not receive a second one:
/// “host:25565:25566” is rejected by Guava, just like a bare IPv6 address.
/// Both halves of the rule matter — a port is needed *and* what precedes it
/// must not be a bracket-less IPv6 address, whose colons don't separate a
/// port.
#[test]
fn a_host_that_already_carries_its_port_is_recognized() {
    assert!(already_has_a_port("mc.example.fr:25565"));
    assert!(already_has_a_port("[2001:db8::1]:25565"));

    // No port at all.
    assert!(!already_has_a_port("mc.example.fr"));
    // What follows the colon isn't a port.
    assert!(!already_has_a_port("mc.example.fr:game"));
    // A bare IPv6 address: its colons don't separate a port, and the last
    // group could pass for a number.
    assert!(!already_has_a_port("2001:db8::25565"));
}
