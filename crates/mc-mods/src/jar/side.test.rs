use super::Side;

#[test]
fn union_of_sides() {
    assert_eq!(Side::Client.union(Side::Client), Side::Client);
    assert_eq!(Side::Client.union(Side::Server), Side::Both);
    assert_eq!(Side::Both.union(Side::Client), Side::Both);
    assert!(Side::Both.includes(Side::Server));
    assert!(!Side::Client.includes(Side::Server));
}

/// These words come from mod manifests and API responses: Modrinth writes
/// "server", NeoForge "DEDICATED_SERVER", and casing varies from one author
/// to another. Missing one puts the mod on the wrong side — so a server that
/// refuses to start, or a client without its mod.
#[test]
fn sides_read_exactly_as_manifests_write_them() {
    assert_eq!(Side::parse("client"), Some(Side::Client));
    assert_eq!(Side::parse("CLIENT"), Some(Side::Client));
    assert_eq!(Side::parse("server"), Some(Side::Server));
    assert_eq!(Side::parse("DEDICATED_SERVER"), Some(Side::Server));
    assert_eq!(Side::parse(" both "), Some(Side::Both));

    // What we can't read isn't guessed: it's up to the caller to decide
    // what to do with a missing side, not this table's job to invent one.
    assert_eq!(Side::parse("both sides"), None);
    assert_eq!(Side::parse(""), None);
}

/// Writing is the inverse of reading, and these strings end up in the
/// lockfile: changing them would make an already-written lock unreadable.
#[test]
fn each_side_writes_as_it_reads() {
    for side in [Side::Client, Side::Server, Side::Both] {
        assert_eq!(Side::parse(side.as_str()), Some(side), "for {side:?}");
    }
    assert_eq!(Side::Client.as_str(), "client");
    assert_eq!(Side::Server.as_str(), "server");
    assert_eq!(Side::Both.as_str(), "both");
}
