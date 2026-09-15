//! Plomberie partagée : téléchargement vérifié et emplacements de données.
//!
//! Installer un modpack, c'est récupérer quelques milliers de fichiers depuis
//! cinq domaines différents. Trois propriétés suffisent à rendre l'opération
//! sûre et relançable :
//!
//! - **vérifié** — chaque fichier est comparé à l'empreinte publiée par sa
//!   source. Un CDN qui renvoie une page d'erreur en HTTP 200 est détecté ici,
//!   pas trois heures plus tard sous la forme d'un crash de NeoForge ;
//! - **idempotent** — un fichier déjà présent *et* conforme n'est pas
//!   retéléchargé. Relancer une installation interrompue reprend où elle en
//!   était, ce qui compte quand il reste 2 500 objets d'assets ;
//! - **atomique** — l'écriture passe par un `.part` renommé à la fin. Une
//!   coupure ne laisse jamais un fichier tronqué que la vérification d'un
//!   prochain passage prendrait pour valide s'il n'y avait pas d'empreinte.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Agent annoncé à toutes les API contactées.
///
/// Modrinth demande explicitement un agent identifiable — `projet/version
/// (contact)` — et limite plus sévèrement les agents anonymes ; Mojang et
/// Adoptium ne l'exigent pas mais le journalisent. Une seule constante pour
/// tout le launcher : c'est ce qui rend un abus traçable jusqu'à nous.
pub const USER_AGENT: &str = "samflix-mc-launcher/0.1 (+https://github.com/samflix-mc/mc-launcher)";

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

/// Racine des données du launcher, selon la convention de chaque système.
///
/// Tout ce que le launcher installe y vit : runtimes Java, instances, caches.
/// Un seul endroit à supprimer pour repartir de zéro, et rien qui traîne dans
/// le répertoire personnel.
pub fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);

    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("samflix-mc");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home.clone() {
            return home
                .join("Library")
                .join("Application Support")
                .join("samflix-mc");
        }
    }

    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(xdg).join("samflix-mc");
    }
    home.unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("samflix-mc")
}

/// Empreinte SHA-1 d'un fichier déjà sur le disque.
pub fn sha1_of_file(path: &Path) -> Result<String> {
    use sha1::{Digest, Sha1};
    Ok(hex::encode(Sha1::digest(std::fs::read(path)?)))
}

/// Ce qu'a fait [`Downloader::to_file`], pour distinguer un vrai
/// téléchargement d'un fichier déjà conforme dans le compte rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetched {
    Downloaded,
    AlreadyPresent,
}

/// Comment décider qu'un fichier déjà présent n'a pas besoin d'être repris.
///
/// La distinction n'est pas cosmétique : les assets d'une version de Minecraft
/// pèsent plus de 800 Mo répartis sur quelques milliers d'objets. Recalculer
/// leur SHA-1 à chaque lancement relit tout le disque pour ne presque jamais
/// rien trouver.
#[derive(Debug, Clone, Copy)]
pub enum Check<'a> {
    /// Empreinte recalculée à chaque passage. Pour ce qui est exécuté — jars,
    /// bibliothèques, runtimes.
    Full(&'a Checksum),
    /// Taille comme première barrière, empreinte vérifiée seulement à
    /// l'écriture. Pour les gros volumes de petits fichiers inertes : un asset
    /// tronqué a la mauvaise taille, et une altération silencieuse à taille
    /// constante donne au pire une texture fausse, jamais du code exécuté.
    /// La vérification exhaustive reste disponible à la demande.
    Quick { sum: &'a Checksum, size: u64 },
    /// Aucune empreinte publiée : seule la présence peut être constatée.
    Presence,
}

impl Check<'_> {
    fn checksum(&self) -> Option<&Checksum> {
        match self {
            Check::Full(sum) => Some(sum),
            Check::Quick { sum, .. } => Some(sum),
            Check::Presence => None,
        }
    }

    /// Le fichier présent peut-il être conservé sans téléchargement ?
    fn accepts_existing(&self, path: &Path) -> bool {
        match self {
            Check::Full(sum) => std::fs::read(path)
                .map(|b| sum.matches(&b))
                .unwrap_or(false),
            Check::Quick { size, .. } => std::fs::metadata(path)
                .map(|m| m.len() == *size)
                .unwrap_or(false),
            Check::Presence => true,
        }
    }
}

pub struct Downloader {
    client: reqwest::Client,
    retries: u32,
}

impl Downloader {
    /// Les deux API publiques utilisées exigent un `User-Agent` identifiant :
    /// Modrinth documente `projet/version (contact)` et applique une limite de
    /// débit par agent, CurseForge journalise l'agent avec la clé.
    pub fn new(user_agent: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .context("construction du client HTTP")?;
        Ok(Self { client, retries: 3 })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Corps de la réponse, avec réessais sur erreur réseau ou 5xx.
    pub async fn bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut last = None;
        for attempt in 0..self.retries {
            if attempt > 0 {
                // Palier court : les 5xx d'un CDN passent en quelques secondes,
                // et un modpack fait des milliers de requêtes.
                tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt))).await;
            }
            match self.try_bytes(url).await {
                Ok(b) => return Ok(b),
                Err(e) => last = Some(e),
            }
        }
        Err(last.expect("au moins une tentative")).with_context(|| format!("GET {url}"))
    }

    async fn try_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!(
                "HTTP {status} : {}",
                body.chars().take(300).collect::<String>()
            );
        }
        Ok(response.bytes().await?.to_vec())
    }

    /// Télécharge `url` vers `dest`, sauf si `dest` satisfait déjà `check`.
    ///
    /// Ce qui est écrit est **toujours** vérifié quand une empreinte existe ;
    /// `check` ne règle que la sévérité du contrôle sur un fichier déjà là.
    pub async fn to_file(&self, url: &str, dest: &Path, check: Check<'_>) -> Result<Fetched> {
        if dest.is_file() && check.accepts_existing(dest) {
            return Ok(Fetched::AlreadyPresent);
        }

        let bytes = self.bytes(url).await?;
        if let Some(sum) = check.checksum() {
            sum.verify(&bytes, &dest.display().to_string())?;
        }
        write_atomic(dest, &bytes)?;
        Ok(Fetched::Downloaded)
    }
}

/// Écrit par `.part` puis renomme : sur le même système de fichiers, le
/// renommage est atomique, donc `dest` n'existe qu'une fois complet.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("création de {}", parent.display()))?;
    }
    let part: PathBuf = dest.with_extension(format!(
        "{}part",
        dest.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    std::fs::write(&part, bytes).with_context(|| format!("écriture de {}", part.display()))?;
    std::fs::rename(&part, dest).with_context(|| format!("renommage vers {}", dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
