use super::parse;

#[test]
fn a_full_response_gives_every_field() {
    let json = r#"{"players":{"online":12,"max":120},"version":{"name":"1.21.1"},"description":"A server"}"#;

    let status = parse(json, 42).unwrap();

    assert_eq!(status.online, Some(12));
    assert_eq!(status.max, Some(120));
    assert_eq!(status.version.as_deref(), Some("1.21.1"));
    assert_eq!(status.latency_ms, 42);
}

#[test]
fn a_response_without_a_version_still_gives_the_players() {
    let json = r#"{"players":{"online":3,"max":10}}"#;

    let status = parse(json, 10).unwrap();

    assert_eq!(status.online, Some(3));
    assert_eq!(status.max, Some(10));
    assert_eq!(status.version, None);
}

#[test]
fn a_response_without_players_still_gives_the_version() {
    let json = r#"{"version":{"name":"1.21.1"},"description":"A server"}"#;

    let status = parse(json, 5).unwrap();

    // `None` and NOT `Some(0)`: the server said nothing about its players,
    // which is not the same as saying nobody is on. The panel shows a dash
    // for the first and "0" for the second.
    assert_eq!(status.online, None);
    assert_eq!(status.max, None);
    assert_eq!(status.version.as_deref(), Some("1.21.1"));
}

#[test]
fn a_response_carrying_only_a_description_still_parses() {
    let json = r#"{"description":"A server"}"#;

    let status = parse(json, 0).unwrap();

    assert_eq!(status.online, None);
    assert_eq!(status.max, None);
    assert_eq!(status.version, None);
}

/// `description` is either a plain string or a tree of chat components:
/// neither shape is modeled, and the probe must not care which one arrives.
#[test]
fn a_component_tree_description_is_ignored_rather_than_rejected() {
    let json = r#"{"description":{"text":"A server","extra":[{"text":"!","color":"gold"}]},"players":{"online":1,"max":2}}"#;

    let status = parse(json, 0).unwrap();

    assert_eq!(status.online, Some(1));
    assert_eq!(status.max, Some(2));
}

#[test]
fn unparsable_json_fails_the_probe() {
    assert!(parse("not json", 0).is_err());
}

/// **An empty server is not a silent server.**
///
/// The distinction this test holds is the whole reason `online` and `max`
/// are `Option`: a server answering "nobody is on, out of a hundred slots"
/// has told us something precise, and the panel must show "0 / 100". A
/// server that omits `players` has told us nothing, and the panel must show
/// a dash. Collapsing both to `0` makes the launcher claim an empty server
/// where it only has silence — and the real development server answers
/// exactly this, `0 / 100`, so the case is not hypothetical.
#[test]
fn zero_players_declared_is_not_the_same_as_no_players_declared() {
    let declared = parse(r#"{"players":{"online":0,"max":100}}"#, 0).unwrap();
    assert_eq!(declared.online, Some(0));
    assert_eq!(declared.max, Some(100));

    let silent = parse(r#"{"players":{}}"#, 0).unwrap();
    assert_eq!(silent.online, None);
    assert_eq!(silent.max, None);
}
