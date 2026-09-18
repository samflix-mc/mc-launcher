use super::{Profile, Session, offline_session};

/// The empty token is the only visible difference from here between a
/// Microsoft session and a local session — and it's the one that decides
/// whether an online server will accept the player.
#[test]
fn a_session_without_a_token_does_not_open_an_online_server() {
    assert!(!offline_session("Sam").is_online());

    let online = Session {
        minecraft_token: "msa-token".into(),
        profile: Profile {
            id: "0123".into(),
            name: "Sam".into(),
        },
    };
    assert!(online.is_online());
}

/// The UUID is hexadecimal without dashes: it's the form the game expects on
/// its command line.
#[test]
fn the_uuid_announced_to_the_game_has_no_dashes() {
    let session = offline_session("Sam");
    assert_eq!(session.profile.id.len(), 32, "{}", session.profile.id);
    assert!(!session.profile.id.contains('-'));
    assert!(session.profile.id.chars().all(|c| c.is_ascii_hexdigit()));
}
