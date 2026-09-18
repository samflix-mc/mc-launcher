use super::worth_announcing;

/// The last pass of a resolution downloads nothing: it just rereads what
/// the previous ones laid down. Announcing those passes would drown out the
/// line that matters; announcing nothing at all would deprive the player of
/// the only sign the install is moving.
#[test]
fn only_a_pass_that_downloads_something_gets_announced() {
    assert!(!worth_announcing(0));
    assert!(worth_announcing(1));
    assert!(worth_announcing(120));
}
