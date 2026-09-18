use super::{Checksum, sha1_of_file, sha512_of_bytes, sha512_of_file};
/// The one pinned in the lockfile when the source publishes nothing: it must
/// equal exactly what `Checksum::Sha512` will verify afterward.
#[test]
fn the_strong_digest_of_a_file_is_the_one_that_will_be_verified() {
    // A directory of our own: tests in the same binary run in parallel, and
    // a neighbor wipes its own on the way out.
    let dir = std::env::temp_dir().join(format!("mc-dl-digest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("test directory");
    let file = dir.join("empty.jar");
    std::fs::write(&file, b"").expect("test file");

    let computed = sha512_of_file(&file).expect("readable digest");
    assert!(Checksum::Sha512(computed.clone()).matches(b""));
    // Empty string vector, verifiable in any tool.
    assert!(computed.starts_with("cf83e1357eefb8bd"));

    assert_eq!(
        sha1_of_file(&file).expect("readable digest"),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn known_digests() {
    // Empty string vectors, verifiable in any tool.
    assert!(Checksum::Sha1("da39a3ee5e6b4b0d3255bfef95601890afd80709".into()).matches(b""));
    assert!(Checksum::Md5("d41d8cd98f00b204e9800998ecf8427e".into()).matches(b""));
    assert!(
        Checksum::Sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into())
            .matches(b"")
    );
}

#[test]
fn the_digest_case_is_ignored() {
    // CurseForge returns its MD5s in uppercase, Modrinth its SHA-1s in
    // lowercase; comparing byte for byte would reject half of both.
    assert!(Checksum::Sha1("DA39A3EE5E6B4B0D3255BFEF95601890AFD80709".into()).matches(b""));
}

#[test]
fn a_wrong_digest_is_rejected() {
    let sum = Checksum::Sha1("0".repeat(40));
    assert!(sum.verify(b"", "test").is_err());
}

/// The digest of bytes, on the standard's test vectors.
///
/// **Nothing verified it.** Two mutants survived it — return the empty
/// string, return anything — and it's the function the comparison between
/// the published lockfile and the one written rests on: a constant digest
/// would say "nothing changed" on every game session, or "everything
/// changed".
///
/// Both inputs are FIPS 180-4's, copied from the publication and not
/// computed by the code under test.
#[test]
fn the_digest_of_bytes_follows_the_standard() {
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

/// And two different inputs don't yield the same digest.
///
/// The property that actually matters for the use made of it: it's the one
/// that decides a lockfile has moved.
#[test]
fn two_different_contents_are_not_confused() {
    assert_ne!(sha512_of_bytes(b"lock v1"), sha512_of_bytes(b"lock v2"));
    // One byte of difference is enough.
    assert_ne!(sha512_of_bytes(b"a"), sha512_of_bytes(b"b"));
}
