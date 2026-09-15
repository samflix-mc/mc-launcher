//! Du manifeste au dossier `mods` : résolution, téléchargement, rattrapage.
//!
//! Le cycle est volontairement itératif plutôt que récursif sur les seules
//! métadonnées :
//!
//! ```text
//!   demandes du manifeste
//!        ↓  résolution (Modrinth, puis CurseForge)
//!   candidats + dépendances déclarées
//!        ↓  téléchargement vérifié
//!   jars sur le disque
//!        ↓  lecture des neoforge.mods.toml
//!   modId fournis / modId exigés
//!        ↓  écart non vide ? → nouveau tour
//!   plan stable
//! ```
//!
//! Le dernier tour est celui qui compte : il attrape les dépendances qu'aucune
//! API ne déclare. C'est le cas courant — un auteur qui ajoute une bibliothèque
//! entre deux versions ne revient pas éditer la fiche de publication — et c'est
//! exactement ce qui fait planter un client au démarrage avec un écran
//! « Missing or unsupported mods ».

use crate::jar::Side;
use crate::{Candidate, Channel, Origin};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Nombre de tours de rattrapage.
///
/// Une chaîne de dépendances implicites dépasse rarement deux niveaux ; la
/// borne protège d'une boucle si deux mods se réclament mutuellement sans que
/// la recherche converge.
const MAX_PASSES: usize = 6;

/// Téléchargements simultanés. Modrinth limite le débit par agent : au-delà
/// d'une poignée de connexions, les 429 coûtent plus de temps qu'ils n'en font
/// gagner.
const PARALLEL_DOWNLOADS: usize = 6;

/// Un mod demandé par le manifeste.
#[derive(Debug, Clone)]
pub struct Request {
    /// Slug, ou identifiant de projet si la source est imposée.
    pub slug: String,
    /// Force la source. Sans valeur : Modrinth, puis CurseForge.
    pub source: Option<Origin>,
    /// Épingle un build précis — `version_id` Modrinth ou `fileId` CurseForge.
    /// C'est ce qui rend une installation reproductible.
    pub file: Option<String>,
    /// Épingle un numéro de version publié, plus lisible qu'un identifiant.
    pub version: Option<String>,
    /// Impose le côté, au lieu de le déduire des métadonnées.
    pub side: Option<Side>,
    /// Canal le plus instable accepté. `release` par défaut.
    pub channel: Option<Channel>,
}

impl Request {
    pub fn new(slug: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            source: None,
            file: None,
            version: None,
            side: None,
            channel: None,
        }
    }
}

/// Pourquoi un mod se retrouve dans le pack.
///
/// Consigné dans le lockfile : sans cette trace, personne ne sait plus, six
/// mois plus tard, si un jar est là par choix ou parce qu'un autre l'exigeait —
/// ni s'il peut être retiré quand son parent l'est.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// Nommé dans le manifeste.
    Explicit,
    /// Dépendance déclarée par l'API de la source.
    Declared { by: String },
    /// Dépendance qu'aucune API n'annonçait, lue dans le descripteur d'un jar.
    Implicit { by: String, mod_id: String },
}

impl Reason {
    pub fn describe(&self) -> String {
        match self {
            Reason::Explicit => "demandé par le manifeste".to_string(),
            Reason::Declared { by } => format!("dépendance déclarée de {by}"),
            Reason::Implicit { by, mod_id } => {
                format!("dépendance implicite : {by} exige « {mod_id} »")
            }
        }
    }
}

/// Un mod résolu, téléchargé et analysé.
#[derive(Debug, Clone)]
pub struct Installed {
    pub candidate: Candidate,
    /// Côté effectif, après combinaison du manifeste, des métadonnées du
    /// projet et de ce que le descripteur du jar réclame.
    pub side: Side,
    pub reason: Reason,
    /// Chemin dans le cache du launcher.
    pub path: PathBuf,
    /// `modId` que ce jar fournit, jars embarqués compris.
    pub provides: BTreeSet<String>,
    /// `modId` que ce jar exige pour démarrer, hors plateforme.
    pub requires: Vec<crate::jar::Requirement>,
}

/// Résultat complet d'une résolution.
#[derive(Debug, Default)]
pub struct Plan {
    pub mods: Vec<Installed>,
    /// Dépendances exigées par un jar qu'aucune source n'a su fournir.
    /// Non bloquant ici : l'appelant décide d'arrêter ou d'avertir.
    pub unresolved: Vec<Unresolved>,
}

#[derive(Debug, Clone)]
pub struct Unresolved {
    pub mod_id: String,
    pub required_by: String,
    pub side: Side,
}

impl Plan {
    pub fn for_side(&self, side: Side) -> impl Iterator<Item = &Installed> {
        self.mods.iter().filter(move |m| m.side.includes(side))
    }
}

/// Les deux sources, interrogées dans l'ordre.
pub struct Registry {
    dl: Arc<mc_dl::Downloader>,
    modrinth: crate::modrinth::Modrinth,
    curseforge: Option<crate::curseforge::CurseForge>,
    cache: PathBuf,
}

impl Registry {
    pub fn new(cache: PathBuf) -> Result<Self> {
        let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT)?);
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::new(dl.clone()),
            curseforge: crate::curseforge::CurseForge::from_env(dl.clone()),
            dl,
            cache,
        })
    }

    pub fn has_curseforge(&self) -> bool {
        self.curseforge.is_some()
    }

    /// Candidats pour un projet, dans la source demandée ou dans l'ordre par
    /// défaut.
    async fn candidates(
        &self,
        id_or_slug: &str,
        source: Option<Origin>,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        if source != Some(Origin::CurseForge) {
            let found = self.modrinth.candidates(id_or_slug, mc, loader).await?;
            if !found.is_empty() || source == Some(Origin::Modrinth) {
                return Ok(found);
            }
        }
        match &self.curseforge {
            Some(cf) => cf.candidates(id_or_slug, mc, loader).await,
            None => Ok(Vec::new()),
        }
    }

    /// Cherche un projet par le `modId` que déclare un jar.
    async fn find_by_mod_id(&self, mod_id: &str, mc: &str, loader: &str) -> Result<Vec<Candidate>> {
        let found = self.modrinth.find_by_mod_id(mod_id, mc, loader).await?;
        if !found.is_empty() {
            return Ok(found);
        }
        match &self.curseforge {
            Some(cf) => cf.find_by_mod_id(mod_id, mc, loader).await,
            None => Ok(Vec::new()),
        }
    }

    async fn pinned(&self, request: &Request, mc: &str, loader: &str) -> Result<Option<Candidate>> {
        let Some(file) = &request.file else {
            return Ok(None);
        };
        // L'épinglage est explicite : si le build n'existe plus, il vaut mieux
        // s'arrêter que retomber en silence sur une autre version — c'est
        // précisément ce que l'épinglage sert à éviter.
        let found = match request.source {
            Some(Origin::CurseForge) => match &self.curseforge {
                Some(cf) => cf.candidate_by_file(file).await?,
                None => bail!(
                    "{} épingle le fichier CurseForge {file}, mais aucune clé d'API n'est configurée",
                    request.slug
                ),
            },
            Some(Origin::Modrinth) => self.modrinth.candidate_by_version(file).await?,
            None => match self.modrinth.candidate_by_version(file).await? {
                Some(found) => Some(found),
                None => match &self.curseforge {
                    Some(cf) => cf.candidate_by_file(file).await?,
                    None => None,
                },
            },
        };

        let found = found.with_context(|| {
            format!("build {file} épinglé pour {} : introuvable", request.slug)
        })?;
        check_compatible(&found, mc, loader, &request.slug)?;
        Ok(Some(found))
    }
}

/// Un build épinglé n'est pas filtré par l'API : il faut vérifier soi-même
/// qu'il correspond bien à la version de Minecraft et au chargeur du pack.
fn check_compatible(candidate: &Candidate, mc: &str, loader: &str, slug: &str) -> Result<()> {
    // Les deux API ont déjà filtré quand on est passé par la liste ; pour un
    // build épinglé, le nom de fichier est le seul indice disponible sans
    // requête supplémentaire, et il est trop peu fiable pour rejeter. On se
    // contente donc de signaler ce qui est manifestement incohérent.
    let haystack = format!(
        "{} {} {}",
        candidate.file_name, candidate.version_number, candidate.display_name
    )
    .to_ascii_lowercase();

    let other_loaders = ["fabric", "quilt"];
    if other_loaders.iter().any(|l| haystack.contains(l)) && !haystack.contains(loader) {
        bail!(
            "le build épinglé pour {slug} ({}) vise un autre chargeur que {loader}",
            candidate.file_name
        );
    }
    let _ = mc;
    Ok(())
}

/// Choisit le meilleur candidat : canal autorisé, puis publication la plus
/// récente.
fn pick(candidates: Vec<Candidate>, request: &Request) -> Option<Candidate> {
    let limit = request.channel.unwrap_or(Channel::Release);

    if let Some(wanted) = &request.version {
        let wanted_lower = wanted.to_ascii_lowercase();
        if let Some(found) = candidates.iter().find(|c| {
            c.version_number.eq_ignore_ascii_case(wanted)
                || c.file_name.to_ascii_lowercase() == wanted_lower
                || c.display_name.eq_ignore_ascii_case(wanted)
        }) {
            return Some(found.clone());
        }
        return None;
    }

    let mut allowed: Vec<Candidate> = candidates
        .into_iter()
        .filter(|c| c.channel.allowed_by(limit))
        .collect();

    // À défaut de release, on accepte ce qui existe : refuser laisserait un
    // pack sans son mod, ce qui est pire qu'une beta signalée dans le lockfile.
    allowed.sort_by(|a, b| b.published.cmp(&a.published));
    allowed.into_iter().next()
}

/// Réglages de la résolution.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Suivre les dépendances annoncées par les API.
    ///
    /// Les désactiver ne casse rien : le rattrapage par lecture des jars
    /// retrouve les mêmes dépendances, simplement un tour plus tard. C'est ce
    /// qui permet de vérifier que ce rattrapage fonctionne — et de s'en
    /// remettre uniquement à ce que le jeu lira, quand une fiche de
    /// publication est fautive.
    pub follow_declared: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            follow_declared: true,
        }
    }
}

/// Résout, télécharge et vérifie l'ensemble du pack.
pub async fn resolve(
    registry: &Registry,
    requests: &[Request],
    mc: &str,
    loader: &str,
) -> Result<Plan> {
    resolve_with(registry, requests, mc, loader, Options::default()).await
}

pub async fn resolve_with(
    registry: &Registry,
    requests: &[Request],
    mc: &str,
    loader: &str,
    options: Options,
) -> Result<Plan> {
    // Clé d'unicité : un projet ne peut être présent qu'une fois. Deux versions
    // du même mod dans `mods` font échouer NeoForge au chargement.
    let mut chosen: BTreeMap<(Origin, String), Installed> = BTreeMap::new();
    let mut plan = Plan::default();

    // --- Tour 1 : ce que le manifeste demande, et ce que les API déclarent ---
    let mut queue: Vec<(Request, Reason)> = requests
        .iter()
        .cloned()
        .map(|r| (r, Reason::Explicit))
        .collect();

    let mut pass = 0;
    loop {
        pass += 1;
        if pass > MAX_PASSES {
            bail!(
                "la résolution ne se stabilise pas après {MAX_PASSES} tours — \
                 dépendances circulaires ou introuvables"
            );
        }

        // Résolution en largeur : les dépendances déclarées rejoignent la file.
        while let Some((request, reason)) = queue.pop() {
            let key = |c: &Candidate| (c.origin, c.project_id.clone());

            let candidate = match registry.pinned(&request, mc, loader).await? {
                Some(pinned) => pinned,
                None => {
                    let found = registry
                        .candidates(&request.slug, request.source, mc, loader)
                        .await?;
                    let had_candidates = !found.is_empty();
                    match pick(found, &request) {
                        Some(c) => c,
                        None if had_candidates => bail!(
                            "{} : aucune version ne correspond{}",
                            request.slug,
                            match (&request.version, request.channel) {
                                (Some(v), _) => format!(" à la version demandée « {v} »"),
                                (None, Some(ch)) =>
                                    format!(" au canal {} ou plus stable", ch.as_str()),
                                _ => " au canal release".to_string(),
                            }
                        ),
                        None => {
                            let hint = if registry.has_curseforge() {
                                ""
                            } else {
                                " (aucune clé CurseForge configurée : seul Modrinth a été consulté)"
                            };
                            bail!(
                                "{} : introuvable pour Minecraft {mc} / {loader}{hint}",
                                request.slug
                            );
                        }
                    }
                }
            };

            if !candidate.redistributable || candidate.url.is_empty() {
                bail!(
                    "{} : l'auteur a désactivé le téléchargement par un launcher tiers. \
                     Récupérer le fichier sur {} et le déposer dans le dossier des apports manuels.",
                    candidate.slug,
                    candidate.page_url.as_deref().unwrap_or("la page du mod")
                );
            }

            let id = key(&candidate);
            if let Some(existing) = chosen.get_mut(&id) {
                // Déjà retenu par une autre branche : on ne retélécharge pas,
                // mais le côté doit couvrir les deux usages.
                existing.side = existing.side.union(side_for(&request, &candidate));
                continue;
            }

            let deps: Vec<_> = if options.follow_declared {
                candidate.declared_deps.clone()
            } else {
                Vec::new()
            };
            let parent = candidate.name.clone();
            let source = candidate.origin;

            chosen.insert(
                id,
                Installed {
                    side: side_for(&request, &candidate),
                    path: PathBuf::new(),
                    provides: BTreeSet::new(),
                    requires: Vec::new(),
                    reason,
                    candidate,
                },
            );

            for dep in deps {
                // Une dépendance se résout dans la source de son parent : un
                // identifiant Modrinth n'existe pas chez CurseForge.
                queue.push((
                    Request {
                        slug: dep.project_id,
                        source: Some(source),
                        file: dep.version_id,
                        version: None,
                        // Le côté d'une dépendance suit celui de son parent, au
                        // minimum ; il sera affiné par le descripteur du jar.
                        side: None,
                        channel: Some(Channel::Beta),
                    },
                    Reason::Declared { by: parent.clone() },
                ));
            }
        }

        // --- Téléchargement, puis lecture de ce que les jars exigent vraiment ---
        download_all(registry, &mut chosen).await?;
        inspect_all(&mut chosen)?;

        // --- Rattrapage : ce qui manque encore ---
        let missing = missing_requirements(&chosen);
        if missing.is_empty() {
            break;
        }

        for (mod_id, required_by, side) in missing {
            let found = registry.find_by_mod_id(&mod_id, mc, loader).await?;
            let request = Request {
                slug: mod_id.clone(),
                source: None,
                file: None,
                version: None,
                side: Some(side),
                channel: Some(Channel::Beta),
            };
            match pick(found, &request) {
                Some(candidate) => queue.push((
                    Request {
                        slug: candidate.project_id.clone(),
                        source: Some(candidate.origin),
                        file: Some(candidate.version_id.clone()),
                        version: None,
                        side: Some(side),
                        channel: Some(Channel::Beta),
                    },
                    Reason::Implicit {
                        by: required_by,
                        mod_id,
                    },
                )),
                // Une dépendance introuvable n'arrête pas tout : elle peut être
                // fournie par un jar non encore analysé, ou relever d'un mod
                // absent des deux plateformes. L'appelant tranche.
                None => plan.unresolved.push(Unresolved {
                    mod_id,
                    required_by,
                    side,
                }),
            }
        }

        if queue.is_empty() {
            break;
        }
        // Un nouveau tour va résoudre la file : les manques déjà consignés
        // pourraient être comblés, on repart d'une liste propre.
        plan.unresolved.clear();
    }

    plan.mods = chosen.into_values().collect();
    plan.mods.sort_by(|a, b| a.candidate.slug.cmp(&b.candidate.slug));
    Ok(plan)
}

/// Côté retenu : le manifeste prime, sinon les métadonnées du projet.
fn side_for(request: &Request, candidate: &Candidate) -> Side {
    request.side.unwrap_or(candidate.project_side)
}

/// Télécharge ce qui n'a pas encore de chemin, en parallèle borné.
async fn download_all(
    registry: &Registry,
    chosen: &mut BTreeMap<(Origin, String), Installed>,
) -> Result<()> {
    use futures_util::stream::{self, StreamExt};

    let todo: Vec<(Origin, String, String, String, Option<mc_dl::Checksum>)> = chosen
        .iter()
        .filter(|(_, m)| m.path.as_os_str().is_empty())
        .map(|(key, m)| {
            (
                key.0,
                key.1.clone(),
                m.candidate.url.clone(),
                m.candidate.file_name.clone(),
                m.candidate.checksum(),
            )
        })
        .collect();

    let results: Vec<Result<((Origin, String), PathBuf)>> = stream::iter(todo)
        .map(|(origin, id, url, file_name, sum)| {
            let dl = registry.dl.clone();
            // Le cache est indexé par source et par projet : deux mods
            // différents publient parfois un jar au même nom.
            let dest = registry
                .cache
                .join(origin.as_str())
                .join(&id)
                .join(&file_name);
            async move {
                dl.to_file(&url, &dest, sum.as_ref())
                    .await
                    .with_context(|| format!("téléchargement de {file_name}"))?;
                Ok(((origin, id), dest))
            }
        })
        .buffer_unordered(PARALLEL_DOWNLOADS)
        .collect()
        .await;

    for result in results {
        let (key, path) = result?;
        if let Some(entry) = chosen.get_mut(&key) {
            entry.path = path;
        }
    }
    Ok(())
}

/// Lit le descripteur de chaque jar fraîchement téléchargé.
fn inspect_all(chosen: &mut BTreeMap<(Origin, String), Installed>) -> Result<()> {
    for entry in chosen.values_mut() {
        if !entry.provides.is_empty() || entry.path.as_os_str().is_empty() {
            continue;
        }
        let info = crate::jar::inspect(&entry.path)?;
        entry.provides = info.provides;
        entry.requires = info.requires;
    }
    Ok(())
}

/// `modId` exigés par au moins un jar et fournis par aucun.
fn missing_requirements(
    chosen: &BTreeMap<(Origin, String), Installed>,
) -> Vec<(String, String, Side)> {
    let provided: BTreeSet<&String> = chosen
        .values()
        .flat_map(|m| m.provides.iter())
        .collect();

    let mut missing: BTreeMap<String, (String, Side)> = BTreeMap::new();
    for entry in chosen.values() {
        for requirement in &entry.requires {
            if provided.contains(&requirement.mod_id) {
                continue;
            }
            let slot = missing
                .entry(requirement.mod_id.clone())
                .or_insert_with(|| (entry.candidate.name.clone(), requirement.side));
            slot.1 = slot.1.union(requirement.side);
        }
    }

    missing
        .into_iter()
        .map(|(mod_id, (by, side))| (mod_id, by, side))
        .collect()
}

/// Recopie les jars du cache vers un dossier `mods`, pour un côté donné.
///
/// Le dossier est d'abord purgé des jars que le plan ne contient plus :
/// un mod retiré du manifeste doit disparaître de l'instance, sans quoi il
/// resterait chargé et ferait diverger le registre du serveur.
pub fn deploy(plan: &Plan, side: Side, mods_dir: &Path) -> Result<Deployed> {
    std::fs::create_dir_all(mods_dir)
        .with_context(|| format!("création de {}", mods_dir.display()))?;

    let wanted: BTreeSet<String> = plan
        .for_side(side)
        .map(|m| m.candidate.file_name.clone())
        .collect();

    let mut removed = Vec::new();
    for entry in std::fs::read_dir(mods_dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jar") || wanted.contains(&name) {
            continue;
        }
        std::fs::remove_file(entry.path())?;
        removed.push(name);
    }

    let mut installed = 0;
    for entry in plan.for_side(side) {
        let dest = mods_dir.join(&entry.candidate.file_name);
        if dest.exists() {
            let same = match &entry.candidate.sha1 {
                Some(expected) => mc_dl::sha1_of_file(&dest)
                    .map(|got| got.eq_ignore_ascii_case(expected))
                    .unwrap_or(false),
                None => true,
            };
            if same {
                installed += 1;
                continue;
            }
            std::fs::remove_file(&dest)?;
        }
        link_or_copy(&entry.path, &dest)?;
        installed += 1;
    }

    Ok(Deployed { installed, removed })
}

#[derive(Debug)]
pub struct Deployed {
    pub installed: usize,
    pub removed: Vec<String>,
}

/// Lien matériel si possible, copie sinon.
///
/// Un pack pèse plusieurs centaines de mégaoctets et le même jar sert souvent
/// au client et au serveur : le lien évite de le stocker trois fois. Il échoue
/// entre systèmes de fichiers différents, d'où le repli.
fn link_or_copy(from: &Path, to: &Path) -> Result<()> {
    if std::fs::hard_link(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)
        .with_context(|| format!("copie de {} vers {}", from.display(), to.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jar::Requirement;

    fn installed(slug: &str, provides: &[&str], requires: &[(&str, Side)]) -> Installed {
        Installed {
            candidate: Candidate {
                origin: Origin::Modrinth,
                project_id: slug.to_string(),
                slug: slug.to_string(),
                name: slug.to_string(),
                version_id: "v".into(),
                version_number: "1.0".into(),
                display_name: "1.0".into(),
                channel: Channel::Release,
                file_name: format!("{slug}.jar"),
                url: format!("https://exemple/{slug}.jar"),
                sha1: None,
                size: 0,
                published: "2025-01-01".into(),
                project_side: Side::Both,
                declared_deps: Vec::new(),
                page_url: None,
                redistributable: true,
            },
            side: Side::Both,
            reason: Reason::Explicit,
            path: PathBuf::from("/cache").join(format!("{slug}.jar")),
            provides: provides.iter().map(|s| s.to_string()).collect(),
            requires: requires
                .iter()
                .map(|(id, side)| Requirement {
                    mod_id: id.to_string(),
                    version_range: None,
                    side: *side,
                })
                .collect(),
        }
    }

    fn map(entries: Vec<Installed>) -> BTreeMap<(Origin, String), Installed> {
        entries
            .into_iter()
            .map(|e| ((e.candidate.origin, e.candidate.project_id.clone()), e))
            .collect()
    }

    #[test]
    fn une_dependance_absente_est_signalee() {
        let chosen = map(vec![installed(
            "attributefix",
            &["attributefix"],
            &[("bookshelf", Side::Both)],
        )]);
        let missing = missing_requirements(&chosen);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].0, "bookshelf");
        assert_eq!(missing[0].1, "attributefix");
    }

    #[test]
    fn une_dependance_fournie_sous_un_autre_slug_ne_manque_pas() {
        // Le modId `bookshelf` est publié sous le slug `bookshelf-lib` : c'est
        // le modId du jar qui fait foi, jamais le nom du projet.
        let chosen = map(vec![
            installed("attributefix", &["attributefix"], &[("bookshelf", Side::Both)]),
            installed("bookshelf-lib", &["bookshelf"], &[]),
        ]);
        assert!(missing_requirements(&chosen).is_empty());
    }

    #[test]
    fn une_dependance_embarquee_par_jarjar_ne_manque_pas() {
        // Le mod embarque sa bibliothèque : l'installer en plus donnerait deux
        // versions du même modId, ce que NeoForge refuse au chargement.
        let chosen = map(vec![installed(
            "un-mod",
            &["unmod", "unelib"],
            &[("unelib", Side::Both)],
        )]);
        assert!(missing_requirements(&chosen).is_empty());
    }

    #[test]
    fn les_cotes_de_deux_demandeurs_sont_fusionnes() {
        let chosen = map(vec![
            installed("a", &["a"], &[("lib", Side::Client)]),
            installed("b", &["b"], &[("lib", Side::Server)]),
        ]);
        let missing = missing_requirements(&chosen);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].2, Side::Both);
    }

    #[test]
    fn le_plan_filtre_par_cote() {
        let mut client_only = installed("embeddium", &["embeddium"], &[]);
        client_only.side = Side::Client;
        let plan = Plan {
            mods: vec![installed("jei", &["jei"], &[]), client_only],
            unresolved: Vec::new(),
        };
        assert_eq!(plan.for_side(Side::Server).count(), 1);
        assert_eq!(plan.for_side(Side::Client).count(), 2);
    }

    #[test]
    fn le_canal_le_plus_recent_est_retenu() {
        let mut vieux = installed("jei", &["jei"], &[]).candidate;
        vieux.published = "2024-01-01".into();
        let mut recent = vieux.clone();
        recent.published = "2025-06-01".into();
        recent.version_number = "19.56".into();

        let choix = pick(vec![vieux, recent], &Request::new("jei")).unwrap();
        assert_eq!(choix.version_number, "19.56");
    }

    #[test]
    fn une_beta_est_ecartee_par_defaut() {
        let mut beta = installed("jei", &["jei"], &[]).candidate;
        beta.channel = Channel::Beta;
        assert!(pick(vec![beta.clone()], &Request::new("jei")).is_none());

        let mut request = Request::new("jei");
        request.channel = Some(Channel::Beta);
        assert!(pick(vec![beta], &request).is_some());
    }

    #[test]
    fn une_version_epinglee_qui_n_existe_pas_ne_retombe_sur_rien() {
        // Silencieusement retomber sur une autre version annulerait tout
        // l'intérêt de l'épinglage.
        let candidate = installed("jei", &["jei"], &[]).candidate;
        let mut request = Request::new("jei");
        request.version = Some("99.99".into());
        assert!(pick(vec![candidate], &request).is_none());
    }
}
