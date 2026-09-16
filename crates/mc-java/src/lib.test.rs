use super::*;

#[test]
fn majeur_des_deux_schemas_de_version() {
    assert_eq!(parse_major("21.0.5+11"), Some(21));
    assert_eq!(parse_major("21"), Some(21));
    assert_eq!(parse_major("17.0.9"), Some(17));
    // Jusqu'à Java 8, le majeur est le deuxième nombre.
    assert_eq!(parse_major("1.8.0_412"), Some(8));
    assert_eq!(parse_major("1.7.0_80"), Some(7));
    assert_eq!(parse_major("22-ea"), Some(22));
    assert_eq!(parse_major(""), None);
}

#[test]
fn le_runtime_gere_est_le_premier_candidat() {
    let dir = Path::new("/tmp/mc-runtime");
    let list = candidates(dir, 21);
    assert_eq!(list[0], managed_home(dir, 21).join("bin").join("java"));
}
