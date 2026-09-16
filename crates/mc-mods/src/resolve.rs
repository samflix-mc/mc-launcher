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
    /// Empreinte connue par ailleurs, typiquement reprise du verrou.
    ///
    /// Certaines sources ne publient pas d'empreinte. Celle qu'un premier
    /// passage a calculée et figée dans le verrou rend les suivants aussi
    /// vérifiables que pour les autres sources.
    pub expected_sha1: Option<String>,
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
            expected_sha1: None,
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
    curseforge_web: crate::curseforge_web::CurseForgeWeb,
    /// Mémorise qu'une clé a été refusée, pour ne pas retenter — ni réavertir —
    /// à chacun des mods qui suivent.
    key_rejected: std::sync::atomic::AtomicBool,
    cache: PathBuf,
}

impl Registry {
    pub fn new(cache: PathBuf) -> Result<Self> {
        let dl = Arc::new(mc_dl::Downloader::new(mc_dl::USER_AGENT)?);
        Ok(Self {
            modrinth: crate::modrinth::Modrinth::new(dl.clone()),
            curseforge: crate::curseforge::CurseForge::from_env(dl.clone()),
            curseforge_web: crate::curseforge_web::CurseForgeWeb::new(dl.clone()),
            key_rejected: std::sync::atomic::AtomicBool::new(false),
            dl,
            cache,
        })
    }

    pub fn has_curseforge(&self) -> bool {
        self.curseforge.is_some()
    }

    /// Candidats pour un projet, dans la source demandée ou dans l'ordre par
    /// défaut.
    ///
    /// Un identifiant **numérique** ne peut venir que de CurseForge : le
    /// proposer à Modrinth ferait une requête vouée à l'échec pour chaque
    /// dépendance résolue.
    async fn candidates(
        &self,
        id_or_slug: &str,
        source: Option<Origin>,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        let numeric = id_or_slug.parse::<u32>().is_ok();

        if source != Some(Origin::CurseForge) && !numeric {
            let found = self.modrinth.candidates(id_or_slug, mc, loader).await?;
            if !found.is_empty() || source == Some(Origin::Modrinth) {
                return Ok(found);
            }
        }
        self.curseforge_any(id_or_slug, mc, loader).await
    }

    /// CurseForge, avec la clé si elle marche, sans elle sinon.
    ///
    /// Une clé refusée ne doit pas tout arrêter : elle expire, elle se révoque,
    /// et le mode sans clé reste capable d'installer. L'avertissement n'est émis
    /// qu'une fois par exécution — répété à chaque mod, il deviendrait du bruit
    /// qu'on cesse de lire.
    async fn curseforge_any(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        if let Some(cf) = &self.curseforge
            && !self.key_rejected.load(std::sync::atomic::Ordering::Relaxed)
        {
            match cf.candidates(id_or_slug, mc, loader).await {
                Ok(found) if !found.is_empty() => return Ok(found),
                Ok(_) => {}
                Err(e) if crate::curseforge::is_key_error(&e) => {
                    self.key_rejected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("  ! {e}\n    Repli sur CurseForge sans clé.");
                }
                Err(e) => return Err(e),
            }
        }
        self.curseforge_web.candidates(id_or_slug, mc, loader).await
    }

    /// Cherche un projet par le `modId` que déclare un jar.
    async fn find_by_mod_id(&self, mod_id: &str, mc: &str, loader: &str) -> Result<Vec<Candidate>> {
        let found = self.modrinth.find_by_mod_id(mod_id, mc, loader).await?;
        if !found.is_empty() {
            return Ok(found);
        }
        if let Some(cf) = &self.curseforge
            && !self.key_rejected.load(std::sync::atomic::Ordering::Relaxed)
        {
            match cf.find_by_mod_id(mod_id, mc, loader).await {
                Ok(found) if !found.is_empty() => return Ok(found),
                Ok(_) => {}
                Err(e) if crate::curseforge::is_key_error(&e) => {
                    self.key_rejected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => return Err(e),
            }
        }
        // Sans clé, la recherche par mot-clé est fermée : seul un `modId` qui
        // est aussi le slug du projet peut aboutir.
        self.curseforge_web
            .find_by_mod_id(mod_id, mc, loader)
            .await
            .or_else(|_| Ok(Vec::new()))
    }

    /// Build CurseForge épinglé, avec la clé si elle marche, sans elle sinon.
    async fn curseforge_file(&self, id_or_slug: &str, file: &str) -> Result<Option<Candidate>> {
        if let Some(cf) = &self.curseforge
            && !self.key_rejected.load(std::sync::atomic::Ordering::Relaxed)
        {
            match cf.candidate_by_file(file).await {
                Ok(Some(found)) => return Ok(Some(found)),
                Ok(None) => {}
                Err(e) if crate::curseforge::is_key_error(&e) => {
                    self.key_rejected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => return Err(e),
            }
        }
        self.curseforge_web
            .candidate_by_file(id_or_slug, file)
            .await
    }

    async fn pinned(&self, request: &Request, mc: &str, loader: &str) -> Result<Option<Candidate>> {
        let Some(file) = &request.file else {
            return Ok(None);
        };
        // L'épinglage est explicite : si le build n'existe plus, il vaut mieux
        // s'arrêter que retomber en silence sur une autre version — c'est
        // précisément ce que l'épinglage sert à éviter.
        let numeric = request.slug.parse::<u32>().is_ok();
        let found = match request.source {
            Some(Origin::CurseForge) => self.curseforge_file(&request.slug, file).await?,
            Some(Origin::Modrinth) => self.modrinth.candidate_by_version(file).await?,
            None if numeric => self.curseforge_file(&request.slug, file).await?,
            None => match self.modrinth.candidate_by_version(file).await? {
                Some(found) => Some(found),
                None => self.curseforge_file(&request.slug, file).await?,
            },
        };

        let found = found
            .with_context(|| format!("build {file} épinglé pour {} : introuvable", request.slug))?;
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

#[tracing::instrument(
    name = "résolution",
    skip(registry, requests, options),
    fields(demandes = requests.len(), mc, loader)
)]
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

            // Une source qui ne publie pas d'empreinte n'interdit pas de
            // vérifier : celle qu'un passage précédent a figée dans le verrou
            // fait foi.
            let mut candidate = candidate;
            if candidate.sha1.is_none() {
                candidate.sha1 = request.expected_sha1.clone();
            }

            tracing::debug!(
                slug = %candidate.slug,
                source = candidate.origin.as_str(),
                version = %candidate.version_number,
                fichier = %candidate.file_name,
                raison = %reason.describe(),
                "mod retenu"
            );

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
                        expected_sha1: None,
                    },
                    Reason::Declared { by: parent.clone() },
                ));
            }
        }

        // --- Téléchargement, puis lecture de ce que les jars exigent vraiment ---
        let a_telecharger = chosen
            .values()
            .filter(|m| m.path.as_os_str().is_empty())
            .count();
        let debut = std::time::Instant::now();
        download_all(registry, &mut chosen).await?;
        inspect_all(&mut chosen)?;
        if a_telecharger > 0 {
            tracing::info!(
                tour = pass,
                jars = a_telecharger,
                duree_ms = debut.elapsed().as_millis(),
                "{a_telecharger} jars téléchargés et analysés en {} ms (tour {pass})",
                debut.elapsed().as_millis()
            );
        }

        // --- Rattrapage : ce qui manque encore ---
        let missing = missing_requirements(&chosen);
        if missing.is_empty() {
            break;
        }

        for (mod_id, required_by, side) in missing {
            // La trace la plus utile du lot : elle nomme une dépendance que ni
            // le manifeste ni l'API n'annonçaient, et sans laquelle le jeu ne
            // démarrerait pas.
            tracing::info!(
                mod_id = %mod_id,
                exige_par = %required_by,
                "Dépendance implicite : {required_by} exige « {mod_id} », qu'aucune API ne déclarait"
            );
            let found = registry.find_by_mod_id(&mod_id, mc, loader).await?;
            let request = Request {
                slug: mod_id.clone(),
                source: None,
                file: None,
                version: None,
                side: Some(side),
                channel: Some(Channel::Beta),
                expected_sha1: None,
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
                        expected_sha1: None,
                    },
                    Reason::Implicit {
                        by: required_by,
                        mod_id,
                    },
                )),
                // Une dépendance introuvable n'arrête pas tout : elle peut être
                // fournie par un jar non encore analysé, ou relever d'un mod
                // absent des deux plateformes. L'appelant tranche.
                None => {
                    tracing::error!(
                        mod_id = %mod_id,
                        exige_par = %required_by,
                        "« {mod_id} », exigé par {required_by}, est introuvable sur toutes les sources"
                    );
                    plan.unresolved.push(Unresolved {
                        mod_id,
                        required_by,
                        side,
                    })
                }
            }
        }

        if queue.is_empty() {
            break;
        }
        // Un nouveau tour va résoudre la file : les manques déjà consignés
        // pourraient être comblés, on repart d'une liste propre.
        plan.unresolved.clear();
    }

    deduplicate_by_mod_id(&mut chosen);
    plan.mods = chosen.into_values().collect();
    plan.mods
        .sort_by(|a, b| a.candidate.slug.cmp(&b.candidate.slug));
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

    struct Job {
        key: (Origin, String),
        url: String,
        file_name: String,
        sum: Option<mc_dl::Checksum>,
        size: u64,
    }

    let todo: Vec<Job> = chosen
        .iter()
        .filter(|(_, m)| m.path.as_os_str().is_empty())
        .map(|(key, m)| Job {
            key: key.clone(),
            url: m.candidate.url.clone(),
            file_name: m.candidate.file_name.clone(),
            sum: m.candidate.checksum(),
            size: m.candidate.size,
        })
        .collect();

    let results: Vec<Result<((Origin, String), PathBuf)>> = stream::iter(todo)
        .map(|job| {
            let dl = registry.dl.clone();
            // Le cache est indexé par source et par projet : deux mods
            // différents publient parfois un jar au même nom.
            let dest = registry
                .cache
                .join(job.key.0.as_str())
                .join(&job.key.1)
                .join(&job.file_name);
            async move {
                // Un jar est du code exécuté : son empreinte est recontrôlée à
                // chaque passage, pas seulement à l'écriture. À défaut
                // d'empreinte, la taille annoncée est le seul garde-fou.
                let check = match (&job.sum, job.size) {
                    (Some(sum), _) => mc_dl::Check::Full(sum),
                    (None, 0) => mc_dl::Check::Presence,
                    (None, size) => mc_dl::Check::Size(size),
                };
                dl.to_file(&job.url, &dest, check)
                    .await
                    .with_context(|| format!("téléchargement de {}", job.file_name))?;
                Ok((job.key, dest))
            }
        })
        .buffer_unordered(PARALLEL_DOWNLOADS)
        .collect()
        .await;

    for result in results {
        let (key, path) = result?;
        if let Some(entry) = chosen.get_mut(&key) {
            // Source sans empreinte : on calcule la nôtre. Elle part dans le
            // verrou, et les installations suivantes seront vérifiées comme
            // toutes les autres.
            if entry.candidate.sha1.is_none() {
                entry.candidate.sha1 = mc_dl::sha1_of_file(&path).ok();
            }
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

/// Écarte les jars qui fournissent un `modId` déjà fourni par un autre.
///
/// Le même mod peut arriver deux fois par deux chemins : demandé par son slug
/// Modrinth, et tiré comme dépendance par son identifiant CurseForge. Les
/// clés de projet diffèrent, donc rien ne les rapproche — sauf le `modId` que
/// les deux jars déclarent. Or deux jars du même `modId` dans `mods` font
/// échouer NeoForge au chargement, avec un message qui n'aide pas.
///
/// En cas de doublon, on garde celui qui a une empreinte publiée, puis le mod
/// demandé explicitement : le plus vérifiable et le plus intentionnel.
fn deduplicate_by_mod_id(chosen: &mut BTreeMap<(Origin, String), Installed>) {
    let mut owner: BTreeMap<String, (Origin, String)> = BTreeMap::new();
    let mut drop_keys: Vec<(Origin, String)> = Vec::new();

    for (key, entry) in chosen.iter() {
        for mod_id in &entry.provides {
            match owner.get(mod_id) {
                None => {
                    owner.insert(mod_id.clone(), key.clone());
                }
                Some(previous) => {
                    let keep_previous = {
                        let other = &chosen[previous];
                        let score = |m: &Installed| {
                            (m.candidate.sha1.is_some(), m.reason == Reason::Explicit)
                        };
                        score(other) >= score(entry)
                    };
                    let loser = if keep_previous {
                        key.clone()
                    } else {
                        let loser = previous.clone();
                        owner.insert(mod_id.clone(), key.clone());
                        loser
                    };
                    if !drop_keys.contains(&loser) {
                        drop_keys.push(loser);
                    }
                }
            }
        }
    }

    for key in drop_keys {
        chosen.remove(&key);
    }
}

/// `modId` exigés par au moins un jar et fournis par aucun.
fn missing_requirements(
    chosen: &BTreeMap<(Origin, String), Installed>,
) -> Vec<(String, String, Side)> {
    let provided: BTreeSet<&String> = chosen.values().flat_map(|m| m.provides.iter()).collect();

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
            installed(
                "attributefix",
                &["attributefix"],
                &[("bookshelf", Side::Both)],
            ),
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
    fn un_meme_mod_venu_de_deux_sources_n_est_garde_qu_une_fois() {
        // Demandé par son slug Modrinth, puis tiré comme dépendance par son
        // identifiant CurseForge : rien ne rapproche les deux clés de projet,
        // sauf le modId. Deux jars du même modId feraient échouer NeoForge.
        let mut depuis_modrinth = installed("jade", &["jade"], &[]);
        depuis_modrinth.candidate.sha1 = Some("aa".into());

        let mut depuis_cf = installed("jade-cf", &["jade"], &[]);
        depuis_cf.candidate.origin = Origin::CurseForge;
        depuis_cf.candidate.sha1 = None;
        depuis_cf.reason = Reason::Declared { by: "autre".into() };

        let mut chosen = map(vec![depuis_modrinth, depuis_cf]);
        deduplicate_by_mod_id(&mut chosen);

        assert_eq!(chosen.len(), 1);
        // Celui qui porte une empreinte est conservé : il est vérifiable.
        assert!(chosen.values().next().unwrap().candidate.sha1.is_some());
    }

    #[test]
    fn a_empreinte_egale_le_mod_demande_l_emporte() {
        let mut explicite = installed("jade", &["jade"], &[]);
        explicite.candidate.sha1 = Some("aa".into());

        let mut dependance = installed("jade-bis", &["jade"], &[]);
        dependance.candidate.sha1 = Some("bb".into());
        dependance.reason = Reason::Declared { by: "autre".into() };

        let mut chosen = map(vec![explicite, dependance]);
        deduplicate_by_mod_id(&mut chosen);

        assert_eq!(chosen.len(), 1);
        assert_eq!(chosen.values().next().unwrap().reason, Reason::Explicit);
    }

    #[test]
    fn deux_mods_distincts_ne_sont_pas_deduplicates() {
        let mut chosen = map(vec![
            installed("jei", &["jei"], &[]),
            installed("jade", &["jade"], &[]),
        ]);
        deduplicate_by_mod_id(&mut chosen);
        assert_eq!(chosen.len(), 2);
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
