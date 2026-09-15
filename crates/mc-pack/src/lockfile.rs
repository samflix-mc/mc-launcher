//! Le verrou : ce qui a réellement été installé, et pourquoi.
//!
//! Le manifeste dit ce qu'on veut, le verrou dit ce qu'on a eu. L'écart entre
//! les deux est exactement ce que la résolution a décidé : la version choisie
//! quand le manifeste n'en imposait aucune, et les dépendances ajoutées
//! d'elles-mêmes.
//!
//! Il sert à deux choses, et chacune justifierait à elle seule de l'écrire :
//!
//! - **rejouer une installation à l'identique**, des mois plus tard, alors que
//!   toutes les versions ont bougé. C'est ce que fait `install --locked` ;
//! - **rendre lisible ce qui a été ajouté sans être demandé**. Six mois après,
//!   personne ne sait plus si un jar est là par choix ou parce qu'un autre
//!   l'exigeait — la ligne `reason` répond.
//!
//! Il se versionne à côté du manifeste, et une modification qu'on n'explique
//! pas dans une revue est un signal.

use anyhow::{Context, Result};
use mc_mods::{Origin, Plan, Side};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub schema: u32,
    pub pack: String,
    /// Date de génération, en UTC.
    pub generated: String,
    pub minecraft: String,
    pub loader: LockedLoader,
    pub java: u32,
    pub mods: Vec<LockedMod>,
    /// Dépendances qu'aucune source n'a su fournir. Vide en temps normal ;
    /// non vide, c'est le premier endroit à regarder quand le jeu refuse de
    /// démarrer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<LockedMissing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedLoader {
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMod {
    pub slug: String,
    pub name: String,
    pub origin: Origin,
    /// Identifiant du projet dans sa source.
    pub project: String,
    /// Identifiant du build. C'est lui qui permet de rejouer l'installation.
    pub file: String,
    pub version: String,
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    pub size: u64,
    pub side: String,
    /// En clair : demandé, dépendance déclarée, ou dépendance implicite.
    pub reason: String,
    /// `modId` fournis par ce jar, jars embarqués compris. Ce sont eux qui
    /// satisfont les dépendances des autres.
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedMissing {
    pub mod_id: String,
    pub required_by: String,
    pub side: String,
}

impl Lockfile {
    pub fn from_plan(
        pack: &str,
        minecraft: &str,
        loader: LockedLoader,
        java: u32,
        plan: &Plan,
    ) -> Lockfile {
        Lockfile {
            schema: crate::manifest::SCHEMA,
            pack: pack.to_string(),
            generated: now_utc(),
            minecraft: minecraft.to_string(),
            loader,
            java,
            mods: plan
                .mods
                .iter()
                .map(|m| LockedMod {
                    slug: m.candidate.slug.clone(),
                    name: m.candidate.name.clone(),
                    origin: m.candidate.origin,
                    project: m.candidate.project_id.clone(),
                    file: m.candidate.version_id.clone(),
                    version: m.candidate.version_number.clone(),
                    file_name: m.candidate.file_name.clone(),
                    sha1: m.candidate.sha1.clone(),
                    size: m.candidate.size,
                    side: m.side.as_str().to_string(),
                    reason: m.reason.describe(),
                    provides: m.provides.iter().cloned().collect(),
                })
                .collect(),
            unresolved: plan
                .unresolved
                .iter()
                .map(|u| LockedMissing {
                    mod_id: u.mod_id.clone(),
                    required_by: u.required_by.clone(),
                    side: u.side.as_str().to_string(),
                })
                .collect(),
        }
    }

    pub fn load(path: &Path) -> Result<Lockfile> {
        let raw =
            std::fs::read(path).with_context(|| format!("lecture du verrou {}", path.display()))?;
        serde_json::from_slice(&raw).with_context(|| format!("verrou {} illisible", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut json = serde_json::to_string_pretty(self)?;
        json.push('\n');
        mc_dl::write_atomic(path, json.as_bytes())
    }

    /// Chemin du verrou associé à un manifeste : `samflix.json` donne
    /// `samflix.lock.json`, côte à côte dans le dépôt.
    pub fn path_for(manifest: &Path) -> std::path::PathBuf {
        let stem = manifest
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "pack".to_string());
        manifest.with_file_name(format!("{stem}.lock.json"))
    }

    /// Reconstruit les demandes à partir du verrou : chaque mod est épinglé sur
    /// le build exact qui avait été retenu, dépendances comprises.
    pub fn requests(&self) -> Vec<mc_mods::Request> {
        self.mods
            .iter()
            .map(|m| mc_mods::Request {
                slug: m.project.clone(),
                source: Some(m.origin),
                file: Some(m.file.clone()),
                version: None,
                side: Side::parse(&m.side),
                channel: None,
            })
            .collect()
    }

    /// Ce qui a changé par rapport à un autre verrou, en clair.
    pub fn diff(&self, previous: &Lockfile) -> Vec<String> {
        let mut lines = Vec::new();
        for current in &self.mods {
            match previous.mods.iter().find(|p| p.project == current.project) {
                None => lines.push(format!("+ {} {}", current.slug, current.version)),
                Some(before) if before.file != current.file => lines.push(format!(
                    "~ {} {} → {}",
                    current.slug, before.version, current.version
                )),
                Some(_) => {}
            }
        }
        for before in &previous.mods {
            if !self.mods.iter().any(|c| c.project == before.project) {
                lines.push(format!("- {} {}", before.slug, before.version));
            }
        }
        lines
    }
}

/// Horodatage ISO 8601 en UTC, sans dépendance à une bibliothèque de dates.
///
/// La conversion jour julien → date civile est l'algorithme de Howard Hinnant,
/// exact sur toute la plage utile et tenant en quelques lignes. Le verrou n'a
/// besoin de rien d'autre : ni fuseau, ni locale, ni secondes intercalaires.
fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_iso8601(secs)
}

fn format_iso8601(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let time = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horodatage_iso8601() {
        assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
        // Année bissextile, 29 février.
        assert_eq!(format_iso8601(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn le_verrou_se_place_a_cote_du_manifeste() {
        assert_eq!(
            Lockfile::path_for(Path::new("packs/samflix.json")),
            Path::new("packs/samflix.lock.json")
        );
    }

    fn locked(slug: &str, file: &str, version: &str) -> LockedMod {
        LockedMod {
            slug: slug.into(),
            name: slug.into(),
            origin: Origin::Modrinth,
            project: slug.into(),
            file: file.into(),
            version: version.into(),
            file_name: format!("{slug}.jar"),
            sha1: None,
            size: 0,
            side: "both".into(),
            reason: "demandé par le manifeste".into(),
            provides: vec![slug.into()],
        }
    }

    fn lock(mods: Vec<LockedMod>) -> Lockfile {
        Lockfile {
            schema: 1,
            pack: "essai".into(),
            generated: "2025-01-01T00:00:00Z".into(),
            minecraft: "1.21.1".into(),
            loader: LockedLoader {
                kind: "neoforge".into(),
                version: "21.1.250".into(),
            },
            java: 21,
            mods,
            unresolved: Vec::new(),
        }
    }

    #[test]
    fn le_diff_dit_ce_qui_a_bouge() {
        let avant = lock(vec![
            locked("jei", "a", "19.51"),
            locked("jade", "b", "15.10"),
        ]);
        let apres = lock(vec![
            locked("jei", "c", "19.56"),
            locked("bookshelf-lib", "d", "21.1.81"),
        ]);

        let lignes = apres.diff(&avant);
        assert!(lignes.contains(&"~ jei 19.51 → 19.56".to_string()));
        assert!(lignes.contains(&"+ bookshelf-lib 21.1.81".to_string()));
        assert!(lignes.contains(&"- jade 15.10".to_string()));
    }

    #[test]
    fn rejouer_un_verrou_epingle_chaque_build() {
        let verrou = lock(vec![locked("jei", "9myHusbW", "19.56")]);
        let requests = verrou.requests();
        assert_eq!(requests[0].file.as_deref(), Some("9myHusbW"));
        assert_eq!(requests[0].source, Some(Origin::Modrinth));
        assert_eq!(requests[0].side, Some(Side::Both));
    }
}
