use super::write_atomic;

#[test]
fn ecriture_atomique_sans_reliquat() {
    let dir = std::env::temp_dir().join(format!("mc-dl-{}", std::process::id()));
    let dest = dir.join("sous/dossier/fichier.jar");
    write_atomic(&dest, b"contenu").unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"contenu");
    // Le `.part` ne doit pas survivre au renommage.
    assert!(!dest.with_extension("jar.part").exists());
    std::fs::remove_dir_all(&dir).ok();
}
