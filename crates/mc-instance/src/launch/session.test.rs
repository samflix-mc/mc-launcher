use super::Session;

#[test]
fn an_offline_session_carries_a_non_empty_token() {
    // The game requires the argument; an empty string breaks parsing.
    let session = Session::offline("Sam", "uuid");
    assert!(!session.token.is_empty());
    assert_eq!(session.user_type, "legacy");
}
