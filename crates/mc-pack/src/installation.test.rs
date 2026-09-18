use super::should_replay;

/// Two reasons to replay a lock, independent of each other: the pack comes
/// from the network — it's its published lock that decides, not the
/// player's machine — or `--locked` requires it on a local manifest.
/// Confusing the two would resolve a published pack again, and the player
/// would not get the versions the network validated.
#[test]
fn a_remote_pack_or_a_required_lock_replays() {
    assert!(should_replay(true, false), "a remote pack replays");
    assert!(should_replay(false, true), "\"--locked\" requires it");
    assert!(should_replay(true, true));

    // A local manifest being edited resolves again: that's the whole point
    // of working on it.
    assert!(!should_replay(false, false));
}
