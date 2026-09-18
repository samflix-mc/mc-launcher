use super::{Checksum, sha1_of_file, sha512_of_bytes, sha512_of_file};
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
        Checksum::Sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into())
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

/// L'empreinte d'octets, sur les vecteurs de la norme.
///
/// **Rien ne la vérifiait.** Deux mutants y survivaient — rendre la chaîne
/// vide, rendre n'importe quoi — et c'est la fonction sur laquelle repose la
/// comparaison du verrou publié à celui qui est posé : une empreinte constante
/// ferait dire « rien n'a changé » à chaque partie, ou « tout a changé ».
///
/// Les deux entrées sont celles de FIPS 180-4, recopiées de la publication et
/// non calculées par le code qu'on éprouve.
#[test]
fn l_empreinte_d_octets_suit_la_norme() {
    assert_eq!(
        sha512_of_bytes(b""),
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce         47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
            .replace(' ', "")
    );
    assert_eq!(
        sha512_of_bytes(b"abc"),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a         2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
            .replace(' ', "")
    );
}

/// Et deux entrées différentes ne donnent pas la même empreinte.
///
/// La propriété qui compte vraiment pour l'usage qu'on en fait : c'est elle
/// qui décide qu'un verrou a bougé.
#[test]
fn deux_contenus_differents_ne_se_confondent_pas() {
    assert_ne!(sha512_of_bytes(b"verrou v1"), sha512_of_bytes(b"verrou v2"));
    // Un octet de différence suffit.
    assert_ne!(sha512_of_bytes(b"a"), sha512_of_bytes(b"b"));
}
