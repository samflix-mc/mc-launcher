use super::usable_home;
use crate::redact::redact;

/// Two values must be rejected, for opposite reasons. An empty variable
/// names nothing — replacing the empty string with "~" would insert a
/// tilde between every character of the log. And "/" prefixes
/// everything: a whole log would become unreadable, system paths
/// included.
#[test]
fn a_home_that_is_empty_or_reduced_to_the_root_is_not_used_as_a_replacement() {
    assert_eq!(usable_home("/home/sam").as_deref(), Some("/home/sam"));
    assert_eq!(usable_home(""), None);
    assert_eq!(usable_home("/"), None);
}

#[test]
fn the_home_directory_becomes_a_tilde() {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return;
    }
    let output = redact(&format!("{home}/.local/share/samflix-mc/logs"));
    assert!(output.starts_with('~'));
    assert!(!output.contains(&home));
}
