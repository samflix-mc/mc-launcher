use super::*;

/// Celle qu'on fige dans le verrou quand la source ne publie rien : elle
/// doit valoir exactement ce que `Checksum::Sha512` vérifiera ensuite.
#[test]
fn l_empreinte_forte_d_un_fichier_est_celle_qu_on_verifiera() {
    // Un répertoire à soi : les tests du même binaire tournent en
    // parallèle, et le voisin efface le sien en partant.
    let dir = std::env::temp_dir().join(format!("mc-dl-empreinte-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("répertoire de test");
    let fichier = dir.join("vide.jar");
    std::fs::write(&fichier, b"").expect("fichier de test");

    let calcule = sha512_of_file(&fichier).expect("empreinte lisible");
    assert!(Checksum::Sha512(calcule.clone()).matches(b""));
    // Vecteur de la chaîne vide, vérifiable dans n'importe quel outil.
    assert!(calcule.starts_with("cf83e1357eefb8bd"));

    assert_eq!(
        sha1_of_file(&fichier).expect("empreinte lisible"),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn empreintes_connues() {
    // Vecteurs de la chaîne vide, vérifiables dans n'importe quel outil.
    assert!(Checksum::Sha1("da39a3ee5e6b4b0d3255bfef95601890afd80709".into()).matches(b""));
    assert!(Checksum::Md5("d41d8cd98f00b204e9800998ecf8427e".into()).matches(b""));
    assert!(
        Checksum::Sha256(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into()
        )
        .matches(b"")
    );
}

#[test]
fn la_casse_de_l_empreinte_est_ignoree() {
    // CurseForge renvoie ses MD5 en majuscules, Modrinth ses SHA-1 en
    // minuscules ; comparer octet à octet rejetterait la moitié des deux.
    assert!(Checksum::Sha1("DA39A3EE5E6B4B0D3255BFEF95601890AFD80709".into()).matches(b""));
}

#[test]
fn une_empreinte_fausse_est_rejetee() {
    let sum = Checksum::Sha1("0".repeat(40));
    assert!(sum.verify(b"", "essai").is_err());
}

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
