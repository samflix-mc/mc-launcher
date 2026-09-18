use super::Channel;

#[test]
fn a_channel_more_stable_than_the_limit_is_accepted() {
    assert!(Channel::Release.allowed_by(Channel::Beta));
    assert!(Channel::Beta.allowed_by(Channel::Beta));
    assert!(!Channel::Alpha.allowed_by(Channel::Beta));
    assert!(!Channel::Beta.allowed_by(Channel::Release));
}

/// These three words come from manifests and API responses. Confusing
/// "beta" with the default would read it as an alpha: a pack that opts into
/// betas would no longer receive them, and a pack that stops at releases
/// would let alphas in. Nothing would break — the mod list would simply be
/// wrong.
#[test]
fn each_channel_reads_under_the_name_manifests_use() {
    assert_eq!(Channel::parse("release"), Channel::Release);
    assert_eq!(Channel::parse("beta"), Channel::Beta);
    assert_eq!(Channel::parse("alpha"), Channel::Alpha);

    // Case and whitespace don't decide anything: both vary from one source
    // to another.
    assert_eq!(Channel::parse(" BETA "), Channel::Beta);
    assert_eq!(Channel::parse("Release"), Channel::Release);

    // What we can't read is treated as the least stable: better to discard
    // out of caution than promote out of ignorance.
    assert_eq!(Channel::parse("nightly"), Channel::Alpha);
    assert_eq!(Channel::parse(""), Channel::Alpha);
}

/// Writing is the inverse of reading, and these strings are the ones that
/// end up in the lockfile: changing them would make an already-written lock
/// unreadable.
#[test]
fn each_channel_writes_as_it_reads() {
    for channel in [Channel::Release, Channel::Beta, Channel::Alpha] {
        assert_eq!(Channel::parse(channel.as_str()), channel, "for {channel:?}");
    }
    assert_eq!(Channel::Release.as_str(), "release");
    assert_eq!(Channel::Beta.as_str(), "beta");
    assert_eq!(Channel::Alpha.as_str(), "alpha");
}
