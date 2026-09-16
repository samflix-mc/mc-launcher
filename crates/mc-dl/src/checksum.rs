//! Les empreintes, telles que les sources les publient.

use anyhow::{Result, bail};
use std::path::Path;

/// Empreinte publiée par une source. Chacune utilise la sienne : Mojang donne
/// du SHA-1, Adoptium du SHA-256, Modrinth les deux, CurseForge du SHA-1 ou du
/// MD5 selon l'ancienneté du fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
    Md5(String),
}

impl Checksum {
    /// Calcule l'empreinte de `bytes` avec le même algorithme.
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
            "{what} : empreinte {} attendue {}, obtenue {got}",
            self.algorithm(),
            self.expected()
        )
    }
}

/// Empreinte SHA-1 d'un fichier déjà sur le disque.
///
/// Conservée pour ce que les sources amont imposent : Mojang adresse tout
/// vanilla par SHA-1, et CurseForge ne publie rien de plus fort.
pub fn sha1_of_file(path: &Path) -> Result<String> {
    use sha1::{Digest, Sha1};
    Ok(hex::encode(Sha1::digest(std::fs::read(path)?)))
}

/// Empreinte SHA-512 d'un fichier déjà sur le disque.
///
/// Celle qu'on calcule quand personne n'en publie : rien n'oblige alors à
/// retenir l'algorithme le plus faible, et c'est elle que le verrou gardera
/// pour toutes les vérifications suivantes.
pub fn sha512_of_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha512};
    Ok(hex::encode(Sha512::digest(std::fs::read(path)?)))
}

#[cfg(test)]
#[path = "checksum.test.rs"]
mod tests;
