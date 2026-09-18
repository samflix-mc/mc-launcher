use super::*;

#[test]
fn a_single_word_name_gives_its_first_two_letters() {
    assert_eq!(seal("Prism"), "PR");
    assert_eq!(seal("samflix"), "SA");
}

#[test]
fn a_multi_word_name_gives_its_initials() {
    assert_eq!(seal("My Network"), "MN");
    assert_eq!(seal("The Big Server"), "TB");
}

#[test]
fn a_dash_separates_two_words_like_a_space() {
    // "samflix-mc" reads as two words, and "SM" abbreviates it better than
    // "SA", which would lose the second half of the name.
    assert_eq!(seal("samflix-mc"), "SM");
    assert_eq!(seal("my_network"), "MN");
}

#[test]
fn a_name_without_a_letter_does_not_give_an_empty_circle() {
    // Unlikely, but an empty seal stands out more than a question mark, and
    // is harder to diagnose.
    assert_eq!(seal(""), "??");
    assert_eq!(seal("— —"), "??");
}

#[test]
fn a_single_letter_name_remains_displayable() {
    assert_eq!(seal("X"), "X");
}

#[test]
fn the_default_name_is_the_network_s() {
    // A build without `MC_LAUNCHER_NOM` must succeed and fall back here.
    // When the variable is set, it wins — which this test cannot verify,
    // since it's read at compile time.
    assert!(!name().is_empty());
}

#[test]
fn the_brand_serializes_with_its_two_fields() {
    let json = serde_json::to_value(Brand::current()).expect("serialization");

    assert!(json["name"].is_string());
    assert_eq!(json["seal"].as_str().map(str::len), Some(2));
}
