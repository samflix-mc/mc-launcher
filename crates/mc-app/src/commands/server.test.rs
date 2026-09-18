//! What's tested here is the one decision this module makes: telling a
//! successful probe from a failed one apart. Reading the local manifest and
//! dialing a socket both need a display server or a real pack on disk to
//! exercise meaningfully — see `mc-app`'s crate doc on why this crate sits
//! outside the mutation scope.

use super::{ServerState, ServerStatus, from_ping};

#[test]
fn a_successful_probe_reports_online_with_its_numbers() {
    let status = from_ping(
        "mc.ggy.info:25565".to_string(),
        Ok(mc_ping::Status {
            online: Some(12),
            max: Some(120),
            version: Some("1.21.1".to_string()),
            latency_ms: 40,
        }),
    );

    assert_eq!(status.state, ServerState::Online);
    assert_eq!(status.host, "mc.ggy.info:25565");
    assert_eq!(status.players, Some(12));
    assert_eq!(status.slots, Some(120));
    assert_eq!(status.version.as_deref(), Some("1.21.1"));
}

/// A closed port and a timed-out socket arrive here the same way — an
/// `Err` — and must leave the panel the same way too: offline, with no
/// player count nobody actually measured.
#[test]
fn a_failed_probe_reports_offline_with_no_numbers() {
    let status = from_ping(
        "mc.ggy.info:25565".to_string(),
        Err(anyhow::anyhow!("connection refused")),
    );

    assert_eq!(status.state, ServerState::Offline);
    assert_eq!(status.host, "mc.ggy.info:25565");
    assert_eq!(status.players, None);
    assert_eq!(status.slots, None);
    assert_eq!(status.version, None);
}

#[test]
fn the_undeclared_state_carries_no_host() {
    let status = ServerStatus::undeclared();

    assert_eq!(status.state, ServerState::Undeclared);
    assert_eq!(status.host, "");
    assert_eq!(status.players, None);
}
