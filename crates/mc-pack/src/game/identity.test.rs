use super::{Identity, choose};

/// The choice is explicit, never guessed: `--nickname` requests an offline
/// session. A silent fallback would let a player onto a server under an
/// identity they didn't choose.
#[tokio::test]
async fn a_given_nickname_opens_an_offline_session() {
    let session = choose(Identity::Offline("Sam".into()))
        .await
        .expect("no network needed");

    assert_eq!(session.name, "Sam");
    assert_eq!(session.user_type, "legacy");
    // The game requires the argument but doesn't validate it offline; an
    // empty string would make argument parsing fail.
    assert!(!session.token.is_empty());
    // The UUID follows the vanilla server rule: the player keeps the same one
    // from one game session to the next, inventory and permissions included.
    assert_eq!(session.uuid, mc_auth::offline_session("Sam").profile.id);
}

#[tokio::test]
async fn the_same_nickname_always_gives_the_same_player() {
    let one = choose(Identity::Offline("Sam".into())).await.unwrap();
    let other = choose(Identity::Offline("Sam".into())).await.unwrap();
    assert_eq!(one.uuid, other.uuid);

    let another = choose(Identity::Offline("Alex".into())).await.unwrap();
    assert_ne!(one.uuid, another.uuid);
}

/// Without `--nickname` and without a registered session, the message must
/// give both ways out: sign in, or play offline.
///
/// Only the message is checked. Triggering it for real would require
/// guaranteeing no session exists on the fixture machine — see the note on
/// [`NO_SESSION`].
#[test]
fn the_refusal_without_a_session_gives_both_ways_out() {
    let message = super::NO_SESSION;

    assert!(message.contains("mc-auth login"), "{message}");
    assert!(message.contains("--nickname"), "{message}");
}
