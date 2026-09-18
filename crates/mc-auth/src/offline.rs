//! A session without Microsoft, for development and for `online-mode=false`
//! servers.

use crate::{Profile, Session};

/// Offline profile, to develop without a Microsoft token.
///
/// As long as the app registration isn't approved, `login_with_xbox`
/// answers 403 and no real session is possible. The rest of the launcher —
/// mod installation, JVM command line, Quick Play, the UI — only needs a
/// nickname and a UUID anyway.
///
/// The UUID follows **exactly** the vanilla server's rule:
/// `UUID.nameUUIDFromBytes("OfflinePlayer:<nickname>")`, a version 3 UUID
/// based on MD5. This is essential: the network's backends run
/// `online-mode=false`, they compute the UUID this way, and a randomly
/// invented UUID would give a different player on every connection —
/// inventory, position and permissions lost.
///
/// Produces no token: the returned session cannot join an online server,
/// only a server in offline mode.
pub fn offline_session(name: &str) -> Session {
    use md5::{Digest, Md5};

    let mut hash: [u8; 16] = Md5::digest(format!("OfflinePlayer:{name}").as_bytes()).into();
    hash[6] = (hash[6] & 0x0f) | 0x30; // version 3
    hash[8] = (hash[8] & 0x3f) | 0x80; // RFC 4122 variant

    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();

    Session {
        // No token: this session does not open an online server.
        minecraft_token: String::new(),
        profile: Profile {
            id: hex,
            name: name.to_owned(),
        },
    }
}

#[cfg(test)]
#[path = "offline.test.rs"]
mod tests;
