//! Digests, as the sources publish them.

use anyhow::{Result, bail};
use std::path::Path;

/// Digest published by a source. Each one uses its own: Mojang gives SHA-1,
/// Adoptium SHA-256, Modrinth both, CurseForge SHA-1 or MD5 depending on how
/// old the file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
    Md5(String),
}

impl Checksum {
    /// Computes the digest of `bytes` with the same algorithm.
    pub fn of(&self, bytes: &[u8]) -> String {
        use md5::Md5;
        use sha1::{Digest, Sha1};
        use sha2::{Sha256, Sha512};

        match self {
            Checksum::Sha1(_) => hex::encode(Sha1::digest(bytes)),
            Checksum::Sha256(_) => hex::encode(Sha256::digest(bytes)),
            Checksum::Sha512(_) => hex::encode(Sha512::digest(bytes)),
            Checksum::Md5(_) => hex::encode(Md5::digest(bytes)),
        }
    }

    pub fn expected(&self) -> &str {
        match self {
            Checksum::Sha1(v) | Checksum::Sha256(v) | Checksum::Sha512(v) | Checksum::Md5(v) => v,
        }
    }

    pub fn algorithm(&self) -> &'static str {
        match self {
            Checksum::Sha1(_) => "SHA-1",
            Checksum::Sha256(_) => "SHA-256",
            Checksum::Sha512(_) => "SHA-512",
            Checksum::Md5(_) => "MD5",
        }
    }

    pub fn matches(&self, bytes: &[u8]) -> bool {
        self.of(bytes).eq_ignore_ascii_case(self.expected())
    }

    pub fn verify(&self, bytes: &[u8], what: &str) -> Result<()> {
        let got = self.of(bytes);
        if got.eq_ignore_ascii_case(self.expected()) {
            return Ok(());
        }
        bail!(
            "{what}: {} digest expected {}, got {got}",
            self.algorithm(),
            self.expected()
        )
    }
}

/// SHA-1 digest of a file already on disk.
///
/// Kept for what upstream sources impose: Mojang addresses everything
/// vanilla by SHA-1, and CurseForge publishes nothing stronger.
pub fn sha1_of_file(path: &Path) -> Result<String> {
    use sha1::{Digest, Sha1};
    Ok(hex::encode(Sha1::digest(std::fs::read(path)?)))
}

/// SHA-512 digest of a file already on disk.
///
/// The one computed when nobody publishes one: nothing then forces settling
/// for the weakest algorithm, and it's the one the lockfile will keep for
/// every check that follows.
pub fn sha512_of_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha512};
    Ok(hex::encode(Sha512::digest(std::fs::read(path)?)))
}

/// SHA-512 digest of bytes that aren't a file.
///
/// The case that brought it into being is comparing the published lockfile
/// to the one that was written: the former arrives over the network and
/// never touched disk. Writing it somewhere just to hash it would amount to
/// installing before deciding whether to install.
pub fn sha512_of_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha512};
    hex::encode(Sha512::digest(bytes))
}

#[cfg(test)]
#[path = "checksum.test.rs"]
mod tests;
