//! Factories shared by this module's test suites.

use std::collections::BTreeMap;

use super::{Loader, Manifest, SCHEMA, Server};

pub(super) fn base() -> Manifest {
    Manifest {
        schema: SCHEMA,
        name: "test".into(),
        version: None,
        minecraft: "1.21.1".into(),
        loader: Loader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: None,
        generation: 0,
        mods: Vec::new(),
        servers: BTreeMap::new(),
    }
}

pub(super) fn with_servers() -> Manifest {
    let mut manifest = base();
    manifest.servers.insert(
        "development".into(),
        Server {
            host: "78.46.100.5".into(),
            port: Some(25566),
        },
    );
    manifest.servers.insert(
        "production".into(),
        Server {
            host: "mc.ggy.info".into(),
            port: Some(25565),
        },
    );
    manifest
}

pub(super) fn server(host: &str, port: Option<u16>) -> Server {
    Server {
        host: host.into(),
        port,
    }
}

pub(super) fn with_key(key: &str, host: &str) -> Manifest {
    let mut manifest = base();
    manifest.servers.insert(
        key.into(),
        Server {
            host: host.into(),
            port: None,
        },
    );
    manifest
}
