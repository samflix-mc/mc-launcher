//! Le manifeste : ce qu'un pack est, écrit une fois et versionné.
//!
//! Un seul fichier décrit l'installation entière — version du jeu, chargeur,
//! version de Java, liste des mods. Il est volontairement court : tout ce qui
//! peut être déduit l'est, et ce qui est écrit à la main est ce qu'un humain a
//! décidé.
//!
//! Chaque mod peut être laissé libre (« la dernière version compatible ») ou
//! **épinglé** sur un build précis. Les deux ont leur place : le pack de
//! développement suit les mises à jour, celui de production ne bouge que
//! lorsqu'on le décide. C'est le rôle de `file`, qui désigne un build exact.

use anyhow::{Context, Result, bail};
use mc_mods::{Channel, Origin, Request, Side};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Version du format, pour pouvoir le faire évoluer sans casser les packs
/// existants.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Version de Minecraft, p. ex. `1.21.1`.
    pub minecraft: String,
    pub loader: Loader,
    /// Version majeure de Java. À défaut, celle qu'exige Mojang pour cette
    /// version du jeu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<u32>,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loader {
    #[serde(rename = "type")]
    pub kind: String,
    /// Version exacte, ou `latest` pour la dernière publiée de la série
    /// correspondant à la version du jeu.
    pub version: String,
}

impl Loader {
    pub fn is_latest(&self) -> bool {
        self.version.eq_ignore_ascii_case("latest")
    }
}

/// Un mod demandé.
///
/// Seul `slug` est obligatoire. Les autres champs servent à sortir du
/// comportement par défaut, et chacun consigne une décision :
/// `file` fige un build, `side` contredit ce que la plateforme annonce,
/// `channel` autorise une préversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Origin>,
    /// Identifiant du build épinglé — `version_id` Modrinth, `fileId`
    /// CurseForge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Numéro de version publié, plus lisible qu'un identifiant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
}

impl ModEntry {
    pub fn to_request(&self) -> Result<Request> {
        let side = match &self.side {
            Some(text) => Some(
                Side::parse(text)
                    .with_context(|| format!("côté inconnu pour {} : « {text} »", self.slug))?,
            ),
            None => None,
        };
        Ok(Request {
            slug: self.slug.clone(),
            source: self.source,
            file: self.file.clone(),
            version: self.version.clone(),
            side,
            channel: self.channel,
            // Le manifeste ne porte pas d'empreinte : elle vient du verrou,
            // qui est justement ce que le manifeste ne veut pas répéter.
            expected_sha1: None,
        })
    }
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest> {
        let raw = std::fs::read(path)
            .with_context(|| format!("lecture du manifeste {}", path.display()))?;
        let manifest: Manifest = serde_json::from_slice(&raw)
            .with_context(|| format!("manifeste {} illisible", path.display()))?;
        manifest.check()?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        mc_dl::write_atomic(path, json.as_bytes())
    }

    /// Refuse un manifeste incohérent avant toute écriture sur le disque.
    ///
    /// Un manifeste fautif découvert après 800 Mo de téléchargement coûte
    /// beaucoup plus qu'une vérification à la lecture.
    fn check(&self) -> Result<()> {
        if self.schema != SCHEMA {
            bail!(
                "manifeste au format {} alors que cette version lit le format {SCHEMA}",
                self.schema
            );
        }
        if self.loader.kind != "neoforge" {
            bail!(
                "seul le chargeur neoforge est géré, manifeste : « {} »",
                self.loader.kind
            );
        }
        let mut seen = std::collections::BTreeSet::new();
        for entry in &self.mods {
            if !seen.insert(entry.slug.to_ascii_lowercase()) {
                bail!("{} apparaît deux fois dans le manifeste", entry.slug);
            }
            if entry.file.is_some() && entry.version.is_some() {
                bail!(
                    "{} épingle à la fois un build (file) et un numéro de version : \
                     les deux se contrediraient",
                    entry.slug
                );
            }
        }
        Ok(())
    }

    pub fn requests(&self) -> Result<Vec<Request>> {
        self.mods.iter().map(ModEntry::to_request).collect()
    }

    /// Version majeure de Java à garantir.
    pub fn java_major(&self, mojang_says: u32) -> u32 {
        self.java.unwrap_or(mojang_says)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Manifest {
        Manifest {
            schema: SCHEMA,
            name: "essai".into(),
            version: None,
            minecraft: "1.21.1".into(),
            loader: Loader {
                kind: "neoforge".into(),
                version: "21.1.250".into(),
            },
            java: None,
            mods: Vec::new(),
        }
    }

    #[test]
    fn un_manifeste_minimal_est_accepte() {
        assert!(base().check().is_ok());
    }

    #[test]
    fn un_format_inconnu_est_refuse() {
        let mut m = base();
        m.schema = 99;
        assert!(m.check().is_err());
    }

    #[test]
    fn un_mod_en_double_est_refuse() {
        let mut m = base();
        m.mods = vec![
            ModEntry {
                slug: "jei".into(),
                source: None,
                file: None,
                version: None,
                side: None,
                channel: None,
            },
            ModEntry {
                slug: "JEI".into(),
                source: None,
                file: None,
                version: None,
                side: None,
                channel: None,
            },
        ];
        assert!(m.check().is_err());
    }

    #[test]
    fn epingler_deux_fois_la_meme_chose_est_refuse() {
        let mut m = base();
        m.mods = vec![ModEntry {
            slug: "jei".into(),
            source: None,
            file: Some("abcd1234".into()),
            version: Some("19.51.0.418".into()),
            side: None,
            channel: None,
        }];
        assert!(m.check().is_err());
    }

    #[test]
    fn le_java_du_manifeste_prime_sur_celui_de_mojang() {
        let mut m = base();
        assert_eq!(m.java_major(21), 21);
        m.java = Some(22);
        assert_eq!(m.java_major(21), 22);
    }

    #[test]
    fn le_cote_est_lu_depuis_le_texte() {
        let entry = ModEntry {
            slug: "embeddium".into(),
            source: None,
            file: None,
            version: None,
            side: Some("client".into()),
            channel: None,
        };
        assert_eq!(entry.to_request().unwrap().side, Some(Side::Client));
    }

    #[test]
    fn un_cote_inconnu_est_refuse() {
        let entry = ModEntry {
            slug: "x".into(),
            source: None,
            file: None,
            version: None,
            side: Some("les-deux".into()),
            channel: None,
        };
        assert!(entry.to_request().is_err());
    }

    #[test]
    fn aller_retour_json() {
        let json = r#"{
            "schema": 1,
            "name": "samflix",
            "minecraft": "1.21.1",
            "loader": { "type": "neoforge", "version": "latest" },
            "mods": [
                { "slug": "jei" },
                { "slug": "jade", "file": "eYz2YBGT", "source": "modrinth" },
                { "slug": "attributefix", "side": "both", "channel": "beta" }
            ]
        }"#;
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        manifest.check().unwrap();
        assert!(manifest.loader.is_latest());
        assert_eq!(manifest.mods.len(), 3);
        assert_eq!(manifest.mods[1].source, Some(Origin::Modrinth));
        assert_eq!(manifest.mods[2].channel, Some(Channel::Beta));
    }
}
