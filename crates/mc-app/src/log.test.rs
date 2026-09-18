use super::level_of;

/// The five names the front end uses, and what they mean.
///
/// Written on both sides: `core/log.ts` doesn't know any others, and a name
/// that diverged would make every line of one level fall back to `info` —
/// that is, a log that no longer distinguishes anything, without an error.
#[test]
fn the_five_levels_are_recognized() {
    assert_eq!(level_of("error"), tracing::Level::ERROR);
    assert_eq!(level_of("warn"), tracing::Level::WARN);
    assert_eq!(level_of("info"), tracing::Level::INFO);
    assert_eq!(level_of("debug"), tracing::Level::DEBUG);
    assert_eq!(level_of("trace"), tracing::Level::TRACE);
}

/// **An unknown level does not lose the line.**
///
/// The opposite — discarding what can't be named — would silently drop a
/// diagnostic message because of a typo, at the exact moment you're looking
/// for why something is missing.
#[test]
fn an_unknown_level_maps_to_info() {
    assert_eq!(level_of("verbose"), tracing::Level::INFO);
    assert_eq!(level_of(""), tracing::Level::INFO);
    assert_eq!(level_of("ERROR"), tracing::Level::INFO);
}
