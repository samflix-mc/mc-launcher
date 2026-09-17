//! Fabriques partagées par les suites de ce module.

use std::collections::BTreeMap;

use super::{Loader, Manifest, SCHEMA, Server};

pub(super) fn base() -> Manifest {
    Manifest {
        schema: SCHEMA,
        name: "essai".into(),
        version: None,
        minecraft: "1.21.1".into(),
        loader: Loader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: None,
        mods: Vec::new(),
        servers: BTreeMap::new(),
    }
}

pub(super) fn avec_serveurs() -> Manifest {
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

pub(super) fn serveur(host: &str, port: Option<u16>) -> Server {
    Server {
        host: host.into(),
        port,
    }
}

pub(super) fn avec_cle(cle: &str, host: &str) -> Manifest {
    let mut manifest = base();
    manifest.servers.insert(
        cle.into(),
        Server {
            host: host.into(),
            port: None,
        },
    );
    manifest
}
