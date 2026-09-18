//! Produire un verrou depuis un plan, l'écrire, le relire.

use std::path::Path;

use anyhow::{Context, Result};

use mc_mods::Plan;

use super::entrees::{LockedMissing, LockedMod};
use super::horodatage::now_utc;
use super::{LockedLoader, Lockfile};

impl Lockfile {
    /// Le verrou d'un plan, adossé au manifeste qui l'a demandé.
    ///
    /// Prend le manifeste entier plutôt que trois de ses champs : le verrou en
    /// reprend le nom, la version et les serveurs, et cette liste s'allongera.
    pub fn from_plan(
        manifest: &crate::manifest::Manifest,
        loader: LockedLoader,
        java: u32,
        plan: &Plan,
    ) -> Lockfile {
        Lockfile {
            schema: crate::manifest::SCHEMA,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            generated: now_utc(),
            minecraft: manifest.minecraft.clone(),
            loader,
            java,
            // La génération vient du MANIFESTE, jamais du verrou précédent :
            // c'est celui qui publie qui décide d'une purge, et le verrou d'à
            // côté ne fait que porter sa décision jusqu'au poste du joueur.
            generation: manifest.generation,
            servers: manifest.servers.clone(),
            mods: plan
                .mods
                .iter()
                .map(|m| LockedMod {
                    slug: m.candidate.slug.clone(),
                    name: m.candidate.name.clone(),
                    source: m.candidate.origin,
                    project: m.candidate.project_id.clone(),
                    file: m.candidate.version_id.clone(),
                    version: m.candidate.version_number.clone(),
                    channel: m.candidate.channel,
                    file_name: m.candidate.file_name.clone(),
                    url: m.candidate.url.clone(),
                    sha1: m.candidate.sha1.clone(),
                    sha512: m.candidate.sha512.clone(),
                    size: m.candidate.size,
                    side: m.side.as_str().to_string(),
                    reason: m.reason.describe(),
                    // Racine *et* embarqués : ce champ sert à `verify` pour
                    // décider qu'une dépendance est satisfaite, jamais à
                    // reconnaître un doublon. Le résolveur, lui, distingue les
                    // deux — deux mods embarquent légitimement la même
                    // bibliothèque.
                    provides: m.fournit().cloned().collect(),
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
        Lockfile::parse(&raw).with_context(|| format!("verrou {} illisible", path.display()))
    }

    /// Lit un verrou qui n'a pas de chemin — celui d'une réponse HTTP.
    pub fn parse(raw: &[u8]) -> Result<Lockfile> {
        Ok(serde_json::from_slice(raw)?)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        mc_dl::write_atomic(path, self.canonique()?.as_bytes())
    }

    /// La forme d'écriture, une fois pour toutes.
    ///
    /// Extraite de `save` parce que [`Self::empreinte`] doit produire
    /// exactement la même chaîne : c'est la seule façon qu'un verrou relu du
    /// disque et un verrou reçu du réseau donnent la même empreinte quand ils
    /// décrivent la même installation.
    fn canonique(&self) -> Result<String> {
        let mut json = serde_json::to_string_pretty(self)?;
        // Un fichier qui se termine par une ligne vide se relit mieux dans une
        // revue — et le verrou se versionne à côté du manifeste.
        json.push('\n');
        Ok(json)
    }

    /// Ce qu'on compare pour décider s'il faut réinstaller.
    ///
    /// ## Pourquoi sur la forme canonique et non sur les octets reçus
    ///
    /// Le verrou publié arrive par HTTP, et l'hôte ne promet aucune mise en
    /// forme : deux espaces d'indentation aujourd'hui, quatre demain, ou une
    /// minification le jour où quelqu'un branche un proxy. Hacher les octets
    /// reçus ferait conclure « le pack a changé » sur un écart d'indentation,
    /// et huit cents mégaoctets repartiraient à chaque partie.
    ///
    /// On resérialise donc des DEUX côtés avec la même fonction. Ce qui est
    /// comparé est alors ce que le verrou DIT, et non comment il est écrit.
    ///
    /// Conséquence à connaître : un champ ajouté à la structure change
    /// l'empreinte de tous les verrous, y compris ceux qui n'ont pas bougé.
    /// C'est le comportement voulu — un champ neuf veut dire que le launcher
    /// sait quelque chose de nouveau sur l'installation, et une vérification
    /// de trop coûte moins cher qu'une vérification manquante.
    pub fn empreinte(&self) -> Result<String> {
        Ok(mc_dl::sha512_of_bytes(self.canonique()?.as_bytes()))
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
}

#[cfg(test)]
#[path = "lecture.test.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "lecture.suite.test.rs"]
mod suite;
