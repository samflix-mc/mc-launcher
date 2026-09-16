use super::parse_descriptor;
use crate::jar::Side;

#[test]
fn le_side_d_une_dependance_est_retenu() {
    let info = parse_descriptor(
        r#"
[[mods]]
modId="skin"

[[dependencies.skin]]
modId="embeddium"
type="required"
side="CLIENT"
"#,
    )
    .unwrap();
    assert_eq!(info.requires[0].side, Side::Client);
}

#[test]
fn un_jar_multi_mods_fusionne_les_cotes_d_une_meme_dependance() {
    let info = parse_descriptor(
        r#"
[[mods]]
modId="a"
[[mods]]
modId="b"

[[dependencies.a]]
modId="lib"
type="required"
side="CLIENT"

[[dependencies.b]]
modId="lib"
type="required"
side="SERVER"
"#,
    )
    .unwrap();
    assert_eq!(info.provides.len(), 2);
    assert_eq!(info.requires.len(), 1);
    assert_eq!(info.requires[0].side, Side::Both);
}
