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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Idem, quand le verrou porte l'empreinte forte.
    pub expected_sha512: Option<String>,
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
            expected_sha512: None,
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

/// Ce qui départage deux branches qui réclament le même projet.
///
/// Le manifeste prime sur ce qu'une API déclare, qui prime sur ce qu'un jar
/// exige ; et à origine égale, une demande épinglée prime sur une demande
/// ouverte. Sans cet ordre, le premier arrivé gardait la place — donc le hasard
/// du parcours décidait de la version installée.
fn autorite(reason: &Reason, request: &Request) -> u8 {
    let origine = match reason {
        Reason::Explicit => 4,
        Reason::Declared { .. } => 2,
        Reason::Implicit { .. } => 0,
    };
    origine + u8::from(request.file.is_some() || request.version.is_some())
}

/// Une exigence lue dans un jar vient-elle de perdre définitivement sa place ?
///
/// Vrai quand la demande est implicite, qu'un demandeur au moins aussi
/// autoritaire tient déjà la clé, et qu'il y tient un autre build. Le tour
/// suivant relèverait le même manque, proposerait le même projet et le
/// reperdrait à l'identique : la file ne se viderait jamais et la résolution
/// mourait sur [`MAX_PASSES`], en accusant des dépendances circulaires qui
/// n'existent pas. Le manque est consigné une fois, et l'installation continue.
fn impasse_implicite(reason: &Reason, entrante: u8, retenue: u8, meme_build: bool) -> bool {
    matches!(reason, Reason::Implicit { .. }) && entrante <= retenue && !meme_build
}

/// Clé d'unicité d'un projet : un mod ne peut être présent qu'une fois.
type Cle = (Origin, String);

/// Une demande en attente de résolution.
struct Demande {
    request: Request,
    reason: Reason,
    /// Le build qui a poussé cette demande, quand elle découle d'un autre.
    ///
    /// La clé, et non le titre que porte [`Reason::Declared`] : « Jade » se
    /// publie sous le même nom chez Modrinth et chez CurseForge, et les deux
    /// projets coexistent dans `chosen` jusqu'à la déduplication finale.
    /// Retirer les dépendances d'un build écarté par son titre emportait donc
    /// celles de son homonyme, que plus rien ne repoussait.
    parent: Option<Cle>,
}

/// File de résolution : le manifeste d'abord, les dépendances ensuite.
///
/// Une seule file suffisait tant qu'on ne regardait que le résultat. Mais elle
/// se vidait en pile : la dépendance d'une demande déjà dépilée passait avant
/// les demandes restantes. Un mod à la fois épinglé par le verrou et dépendance
/// d'un autre était donc résolu sans son épinglage, et la demande épinglée
/// arrivait sur une clé déjà prise.
#[derive(Default)]
struct FileDeResolution {
    manifeste: Vec<Demande>,
    derivees: Vec<Demande>,
}

impl FileDeResolution {
    fn pousser(&mut self, request: Request, reason: Reason, parent: Option<Cle>) {
        let demande = Demande {
            request,
            reason,
            parent,
        };
        match demande.reason {
            Reason::Explicit => self.manifeste.push(demande),
            _ => self.derivees.push(demande),
        }
    }

    /// Dépile en profondeur, mais jamais une dépendance tant que le manifeste
    /// n'est pas entièrement traité.
    fn suivante(&mut self) -> Option<Demande> {
        self.manifeste.pop().or_else(|| self.derivees.pop())
    }

    /// Retire de la file les dépendances qu'un build avait déclarées.
    ///
    /// Appelé quand un build en remplace un autre : les dépendances du build
    /// écarté n'ont plus de demandeur. Les laisser ferait installer des jars
    /// que plus rien ne réclame — et le verrou les consignerait comme
    /// dépendances d'une version qui n'est pas celle retenue.
    ///
    /// Ne rattrape que ce qui est encore en file. Une dépendance de l'ancien
    /// build déjà résolue à un tour précédent y reste : la retirer demanderait
    /// de savoir qui d'autre s'appuie dessus, donc de tenir le graphe inverse.
    /// L'écart est borné — un jar de bibliothèque en trop, que NeoForge charge
    /// sans se plaindre — et sans commune mesure avec le défaut d'en face,
    /// qui était d'installer le mauvais build épinglé.
    fn oublier_dependances_de(&mut self, parent: &Cle) {
        self.derivees
            .retain(|demande| demande.parent.as_ref() != Some(parent));
    }

    fn est_vide(&self) -> bool {
        self.manifeste.is_empty() && self.derivees.is_empty()
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
    /// Au nom de quoi ce build occupe la place — voir [`autorite`].
    ///
    /// Porté par l'entrée retenue plutôt que par une seconde table indexée de
    /// la même façon : deux tables à tenir en phase, c'est une occasion de les
    /// laisser diverger, et `reason` part dans le verrou.
    autorite: u8,
}

/// Résultat complet d'une résolution.
#[derive(Debug, Default)]
pub struct Plan {
    pub mods: Vec<Installed>,
    /// Dépendances exigées par un jar qu'aucune source n'a su fournir.
    /// Non bloquant ici : l'appelant décide d'arrêter ou d'avertir.
    pub unresolved: Vec<Unresolved>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Cherche un fournisseur pour chaque `modId` qu'aucune API n'annonçait.
///
/// Sortie de [`resolve_with`] : ce rattrapage ne touche ni à la table des
/// retenus ni au décompte des tours, il ne fait qu'alimenter la file — ou la
/// liste des manques, quand personne ne peut fournir.
async fn rattraper(
    registry: &Registry,
    manques: Vec<(String, String, Side)>,
    mc: &str,
    loader: &str,
    impasses: &BTreeSet<Cle>,
    queue: &mut FileDeResolution,
    unresolved: &mut Vec<Unresolved>,
) -> Result<()> {
    for (mod_id, required_by, side) in manques {
        // La trace la plus utile du lot : elle nomme une dépendance que ni le
        // manifeste ni l'API n'annonçaient, et sans laquelle le jeu ne
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
            expected_sha512: None,
        };
        match suite_du_rattrapage(pick(found, &request), impasses, mod_id, required_by, side) {
            Rattrapage::Demander(request, reason) => queue.pousser(request, reason, None),
            Rattrapage::Renoncer(manque) => unresolved.push(manque),
        }
    }

    Ok(())
}

/// Ce qu'il advient d'une exigence implicite, une fois qu'on a cherché qui
/// pourrait la fournir.
#[derive(Debug, PartialEq, Eq)]
enum Rattrapage {
    /// Un build fournit le `modId` : sa demande rejoint la file.
    Demander(Request, Reason),
    /// Personne ne le fournira par cette voie ; le manque est consigné et
    /// l'installation continue — le jeu démarrera sans, ou pas du tout, mais
    /// c'est à l'appelant d'en juger, pas au résolveur.
    Renoncer(Unresolved),
}

/// Décide sans rien demander au réseau, et c'est tout l'intérêt : la recherche
/// du fournisseur est faite, ce qui reste est un jugement sur ce qu'elle a
/// rendu — jugement dont dépend la terminaison de la résolution.
fn suite_du_rattrapage(
    trouve: Option<Candidate>,
    impasses: &BTreeSet<Cle>,
    mod_id: String,
    required_by: String,
    side: Side,
) -> Rattrapage {
    // Le projet a déjà perdu cet arbitrage : le reproposer relancerait un tour
    // identique, jusqu'à épuisement de [`MAX_PASSES`].
    let impasse = trouve
        .as_ref()
        .is_some_and(|c| impasses.contains(&(c.origin, c.project_id.clone())));

    match trouve {
        Some(candidate) if !impasse => Rattrapage::Demander(
            Request {
                slug: candidate.project_id.clone(),
                source: Some(candidate.origin),
                file: Some(candidate.version_id.clone()),
                version: None,
                side: Some(side),
                channel: Some(Channel::Beta),
                expected_sha1: None,
                expected_sha512: None,
            },
            Reason::Implicit {
                by: required_by,
                mod_id,
            },
        ),
        autre => {
            match &autre {
                Some(candidate) => tracing::error!(
                    mod_id = %mod_id,
                    exige_par = %required_by,
                    projet = %candidate.slug,
                    "« {mod_id} », exigé par {required_by}, ne serait fourni que par \
                     « {} » — dont la place est tenue par un build qu'une demande plus \
                     autoritaire impose",
                    candidate.slug
                ),
                None => tracing::error!(
                    mod_id = %mod_id,
                    exige_par = %required_by,
                    "« {mod_id} », exigé par {required_by}, est introuvable sur toutes les sources"
                ),
            }
            Rattrapage::Renoncer(Unresolved {
                mod_id,
                required_by,
                side,
            })
        }
    }
}

/// Le build à retenir pour une demande, ou l'explication de son absence.
///
/// Sortie de [`resolve_with`], dont elle représentait le gros du volume : trois
/// impasses — version demandée introuvable, projet introuvable, téléchargement
/// interdit par l'auteur — qui n'ont besoin que de la demande pour être
/// décrites, et dont aucune ne regarde l'état de la résolution.
async fn choisir_build(
    registry: &Registry,
    request: &Request,
    mc: &str,
    loader: &str,
) -> Result<Candidate> {
    if let Some(pinned) = registry.pinned(request, mc, loader).await? {
        return Ok(pinned);
    }

    let found = registry
        .candidates(&request.slug, request.source, mc, loader)
        .await?;
    trancher(found, request, mc, loader, registry.has_curseforge())
}

/// Ce qu'on retient d'une liste de candidats, ou pourquoi on ne retient rien.
///
/// Séparé de [`choisir_build`] pour une raison simple : le jugement ne dépend
/// que de ce que la source a répondu, jamais du réseau. Mêlé à l'appel, il
/// était intestable, et ce sont pourtant ces trois messages que le joueur lira
/// le jour où son pack ne s'installe pas.
fn trancher(
    found: Vec<Candidate>,
    request: &Request,
    mc: &str,
    loader: &str,
    has_curseforge: bool,
) -> Result<Candidate> {
    let had_candidates = !found.is_empty();

    let candidate = match pick(found, request) {
        Some(c) => c,
        // Le projet existe, mais rien n'y correspond : dire quoi, sans quoi le
        // message envoie chercher un mod absent alors qu'il est sous les yeux.
        None if had_candidates => bail!(
            "{} : aucune version ne correspond{}",
            request.slug,
            match (&request.version, request.channel) {
                (Some(v), _) => format!(" à la version demandée « {v} »"),
                (None, Some(ch)) => format!(" au canal {} ou plus stable", ch.as_str()),
                _ => " au canal release".to_string(),
            }
        ),
        None => {
            let hint = if has_curseforge {
                ""
            } else {
                " (aucune clé CurseForge configurée : seul Modrinth a été consulté)"
            };
            bail!(
                "{} : introuvable pour Minecraft {mc} / {loader}{hint}",
                request.slug
            );
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

    Ok(candidate)
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
    let mut chosen: BTreeMap<Cle, Installed> = BTreeMap::new();
    let mut plan = Plan::default();
    // Les projets qu'une exigence implicite a tenté de prendre, sans l'emporter.
    // Voir [`impasse_implicite`] : sans cette mémoire, le rattrapage repousse
    // indéfiniment une demande qui reperd le même arbitrage.
    let mut impasses: BTreeSet<Cle> = BTreeSet::new();

    // --- Tour 1 : ce que le manifeste demande, et ce que les API déclarent ---
    let mut queue = FileDeResolution::default();
    for request in requests {
        queue.pousser(request.clone(), Reason::Explicit, None);
    }

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
        while let Some(Demande {
            request, reason, ..
        }) = queue.suivante()
        {
            let key = |c: &Candidate| (c.origin, c.project_id.clone());

            let candidate = choisir_build(registry, &request, mc, loader).await?;

            // Une source qui ne publie pas d'empreinte n'interdit pas de
            // vérifier : celle qu'un passage précédent a figée dans le verrou
            // fait foi.
            let mut candidate = candidate;
            if candidate.sha1.is_none() {
                candidate.sha1 = request.expected_sha1.clone();
            }
            if candidate.sha512.is_none() {
                candidate.sha512 = request.expected_sha512.clone();
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
            let entrante = autorite(&reason, &request);
            let mut cote = side_for(&request, &candidate);

            if let Some(existing) = chosen.get_mut(&id) {
                cote = existing.side.union(cote);

                // Déjà retenu par une branche au moins aussi autoritaire, ou
                // les deux désignent le même build : on ne retélécharge pas,
                // mais le côté doit couvrir les deux usages.
                let meme_build = existing.candidate.version_id == candidate.version_id;
                if impasse_implicite(&reason, entrante, existing.autorite, meme_build) {
                    impasses.insert(id.clone());
                }

                if entrante <= existing.autorite || meme_build {
                    existing.side = cote;
                    // Même build, demandeur plus autoritaire : c'est lui qui
                    // répond désormais de sa présence. Sans cette reprise, le
                    // verrou continuait de nommer le premier venu — celui qu'on
                    // relit pour savoir si un jar peut être retiré.
                    if entrante > existing.autorite {
                        existing.autorite = entrante;
                        existing.reason = reason;
                    }
                    continue;
                }

                // Une demande plus autoritaire arrive après coup. Se contenter
                // de fusionner les côtés reviendrait à installer le build de
                // l'autre branche en gardant le silence : le joueur n'aurait
                // pas la version que le verrou promet, et rien ne le dirait.
                tracing::warn!(
                    slug = %candidate.slug,
                    ecartee = %existing.candidate.version_number,
                    retenue = %candidate.version_number,
                    "« {} » : {} impose {}, qui remplace {} retenue jusqu'ici",
                    candidate.slug,
                    reason.describe(),
                    candidate.version_number,
                    existing.candidate.version_number,
                );
                // Les dépendances déclarées par le build écarté n'ont plus de
                // demandeur : celles du build qui l'emporte vont être poussées
                // juste après, et peuvent être tout autres.
                queue.oublier_dependances_de(&id);

                // L'insertion ci-dessous écrase l'entrée : le chemin repart
                // vide, donc le bon jar sera téléchargé, et les dépendances du
                // build qui l'emporte repassent par la file.
            }

            let deps: Vec<_> = if options.follow_declared {
                candidate.declared_deps.clone()
            } else {
                Vec::new()
            };
            let parent = candidate.name.clone();
            let source = candidate.origin;

            chosen.insert(
                id.clone(),
                Installed {
                    side: cote,
                    path: PathBuf::new(),
                    provides: BTreeSet::new(),
                    requires: Vec::new(),
                    autorite: entrante,
                    reason,
                    candidate,
                },
            );

            for dep in deps {
                // Une dépendance se résout dans la source de son parent : un
                // identifiant Modrinth n'existe pas chez CurseForge.
                queue.pousser(
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
                        expected_sha512: None,
                    },
                    Reason::Declared { by: parent.clone() },
                    Some(id.clone()),
                );
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

        rattraper(
            registry,
            missing,
            mc,
            loader,
            &impasses,
            &mut queue,
            &mut plan.unresolved,
        )
        .await?;

        if queue.est_vide() {
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
            if entry.candidate.checksum().is_none() {
                entry.candidate.sha512 = mc_dl::sha512_of_file(&path).ok();
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
                            (
                                m.candidate.checksum().is_some(),
                                m.reason == Reason::Explicit,
                            )
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
            let same = match entry.candidate.checksum() {
                Some(attendue) => std::fs::read(&dest)
                    .map(|bytes| attendue.matches(&bytes))
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

    /// Le scénario qui faisait mourir la résolution sur `MAX_PASSES` : un
    /// demandeur autoritaire impose un build qui ne fournit pas le `modId`
    /// qu'un jar exige, et l'exigence rejouait sa demande perdue à chaque tour.
    #[test]
    fn une_exigence_implicite_qui_reperd_sa_place_est_une_impasse() {
        let implicite = Reason::Implicit {
            by: "build_B".into(),
            mod_id: "libfoo".into(),
        };
        assert!(impasse_implicite(&implicite, 1, 3, false));
    }

    #[test]
    fn une_exigence_implicite_qui_emporte_la_place_n_est_pas_une_impasse() {
        let implicite = Reason::Implicit {
            by: "build_B".into(),
            mod_id: "libfoo".into(),
        };
        assert!(!impasse_implicite(&implicite, 3, 1, false));
    }

    /// Perdre l'arbitrage contre un demandeur qui désigne le même jar ne prive
    /// de rien : le `modId` sera fourni, la demande n'a plus lieu d'être.
    #[test]
    fn perdre_contre_le_meme_build_n_est_pas_une_impasse() {
        let implicite = Reason::Implicit {
            by: "build_B".into(),
            mod_id: "libfoo".into(),
        };
        assert!(!impasse_implicite(&implicite, 1, 3, true));
    }

    /// Une dépendance déclarée écartée est repoussée par son parent quand il
    /// est lui-même remplacé ; elle ne se rejoue pas d'elle-même, et n'a donc
    /// pas à être retenue comme impasse.
    #[test]
    fn seules_les_exigences_implicites_font_impasse() {
        let declaree = Reason::Declared { by: "Jade".into() };
        assert!(!impasse_implicite(&declaree, 1, 3, false));
        assert!(!impasse_implicite(&Reason::Explicit, 1, 3, false));
    }

    fn candidat(slug: &str, version: &str) -> Candidate {
        Candidate {
            origin: Origin::Modrinth,
            project_id: format!("{slug}-id"),
            slug: slug.to_string(),
            name: slug.to_string(),
            version_id: format!("{slug}-{version}"),
            version_number: version.to_string(),
            display_name: version.to_string(),
            channel: Channel::Release,
            file_name: format!("{slug}.jar"),
            url: format!("https://exemple.invalid/{slug}.jar"),
            sha1: None,
            sha512: None,
            size: 0,
            published: "2025-01-01".into(),
            project_side: Side::Both,
            declared_deps: Vec::new(),
            page_url: Some(format!("https://modrinth.com/mod/{slug}")),
            redistributable: true,
        }
    }

    /// Sans clé CurseForge, la moitié des sources n'a pas été interrogée : le
    /// dire évite de chercher pourquoi un mod « n'existe pas ».
    #[test]
    fn projet_introuvable_signale_la_source_non_consultee() {
        let erreur = trancher(
            Vec::new(),
            &Request::new("jade"),
            "1.21.1",
            "neoforge",
            false,
        )
        .expect_err("aucun candidat");
        let texte = erreur.to_string();
        assert!(
            texte.contains("introuvable pour Minecraft 1.21.1 / neoforge"),
            "{texte}"
        );
        assert!(
            texte.contains("aucune clé CurseForge configurée"),
            "{texte}"
        );

        let avec_cle = trancher(
            Vec::new(),
            &Request::new("jade"),
            "1.21.1",
            "neoforge",
            true,
        )
        .expect_err("aucun candidat");
        assert!(!avec_cle.to_string().contains("CurseForge configurée"));
    }

    /// Le projet existe, la version demandée non : deux situations qu'un même
    /// message confondrait, alors qu'elles n'appellent pas la même correction.
    #[test]
    fn version_demandee_absente_le_dit_plutot_que_introuvable() {
        let mut request = Request::new("jade");
        request.version = Some("99.0.0".into());
        let erreur = trancher(
            vec![candidat("jade", "15.10.6")],
            &request,
            "1.21.1",
            "neoforge",
            true,
        )
        .expect_err("version absente");
        let texte = erreur.to_string();
        assert!(texte.contains("aucune version ne correspond"), "{texte}");
        assert!(texte.contains("« 99.0.0 »"), "{texte}");
    }

    #[test]
    fn canal_trop_strict_nomme_le_canal() {
        let mut candidats = vec![candidat("jade", "15.10.6")];
        candidats[0].channel = Channel::Beta;
        let mut request = Request::new("jade");
        request.channel = Some(Channel::Release);
        let erreur =
            trancher(candidats, &request, "1.21.1", "neoforge", true).expect_err("aucune release");
        assert!(erreur.to_string().contains("au canal release"), "{erreur}");
    }

    /// Un auteur peut interdire le téléchargement par un tiers. Ce n'est pas
    /// une panne : le fichier existe, il faut aller le chercher à la main, et
    /// le message doit dire où.
    #[test]
    fn telechargement_interdit_renvoie_vers_la_page_du_mod() {
        let mut interdit = candidat("jade", "15.10.6");
        interdit.redistributable = false;
        let erreur = trancher(
            vec![interdit],
            &Request::new("jade"),
            "1.21.1",
            "neoforge",
            true,
        )
        .expect_err("non redistribuable");
        let texte = erreur.to_string();
        assert!(texte.contains("apports manuels"), "{texte}");
        assert!(texte.contains("https://modrinth.com/mod/jade"), "{texte}");
    }

    /// Une URL vide vaut un téléchargement impossible : la distinction ne se
    /// voit qu'au moment de télécharger, trop tard pour l'expliquer.
    #[test]
    fn url_vide_vaut_telechargement_interdit() {
        let mut sans_url = candidat("jade", "15.10.6");
        sans_url.url = String::new();
        let erreur = trancher(
            vec![sans_url],
            &Request::new("jade"),
            "1.21.1",
            "neoforge",
            true,
        )
        .expect_err("url vide");
        assert!(erreur.to_string().contains("apports manuels"));
    }

    #[test]
    fn le_build_retenu_est_rendu_tel_quel() {
        let retenu = trancher(
            vec![candidat("jade", "15.10.6")],
            &Request::new("jade"),
            "1.21.1",
            "neoforge",
            true,
        )
        .expect("un build retenu");
        assert_eq!(retenu.version_number, "15.10.6");
    }

    /// Le cas nominal du rattrapage : un jar exige un modId que personne n'a
    /// déclaré, on trouve qui le fournit, et sa demande rejoint la file.
    #[test]
    fn un_fournisseur_trouve_devient_une_demande_implicite() {
        let suite = suite_du_rattrapage(
            Some(candidat("bookshelf-lib", "21.1.81")),
            &BTreeSet::new(),
            "bookshelf".into(),
            "attributefix".into(),
            Side::Both,
        );
        match suite {
            Rattrapage::Demander(request, reason) => {
                assert_eq!(request.slug, "bookshelf-lib-id");
                assert_eq!(request.source, Some(Origin::Modrinth));
                assert_eq!(request.file.as_deref(), Some("bookshelf-lib-21.1.81"));
                assert_eq!(
                    reason,
                    Reason::Implicit {
                        by: "attributefix".into(),
                        mod_id: "bookshelf".into()
                    }
                );
            }
            autre => panic!("attendu une demande, obtenu {autre:?}"),
        }
    }

    /// Le cas qui faisait tourner la résolution jusqu'à MAX_PASSES : le seul
    /// fournisseur occupe une clé qu'une demande plus autoritaire tient déjà.
    /// Redemander ne changerait rien, donc on consigne le manque.
    #[test]
    fn un_fournisseur_en_impasse_est_consigne_au_lieu_d_etre_redemande() {
        let mut impasses = BTreeSet::new();
        impasses.insert((Origin::Modrinth, "bookshelf-lib-id".to_string()));
        let suite = suite_du_rattrapage(
            Some(candidat("bookshelf-lib", "21.1.81")),
            &impasses,
            "bookshelf".into(),
            "attributefix".into(),
            Side::Both,
        );
        assert_eq!(
            suite,
            Rattrapage::Renoncer(Unresolved {
                mod_id: "bookshelf".into(),
                required_by: "attributefix".into(),
                side: Side::Both,
            })
        );
    }

    #[test]
    fn un_modid_que_personne_ne_fournit_est_consigne() {
        let suite = suite_du_rattrapage(
            None,
            &BTreeSet::new(),
            "libfoo".into(),
            "build_b".into(),
            Side::Client,
        );
        assert_eq!(
            suite,
            Rattrapage::Renoncer(Unresolved {
                mod_id: "libfoo".into(),
                required_by: "build_b".into(),
                side: Side::Client,
            })
        );
    }

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
                sha512: None,
                size: 0,
                published: "2025-01-01".into(),
                project_side: Side::Both,
                declared_deps: Vec::new(),
                page_url: None,
                redistributable: true,
            },
            side: Side::Both,
            reason: Reason::Explicit,
            autorite: 4,
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

    fn cle(source: Origin, projet: &str) -> Cle {
        (source, projet.to_string())
    }

    #[test]
    fn le_manifeste_passe_avant_les_dependances_meme_poussees_en_cours_de_route() {
        // Le défaut d'origine tenait entièrement ici. En pile unique, la
        // dépendance poussée par « b » passait avant la demande « a » restante :
        // « a » était résolu sans son épinglage, et la demande épinglée trouvait
        // ensuite la clé prise. Le joueur installait un autre build que celui du
        // verrou, sans que rien ne le signale.
        let mut queue = FileDeResolution::default();
        queue.pousser(Request::new("a"), Reason::Explicit, None);
        queue.pousser(Request::new("b"), Reason::Explicit, None);

        let premier = queue.suivante().unwrap();
        assert_eq!(premier.request.slug, "b");
        queue.pousser(
            Request::new("a"),
            Reason::Declared { by: "b".into() },
            Some(cle(Origin::Modrinth, "b")),
        );

        let ensuite = queue.suivante().unwrap();
        assert_eq!(ensuite.request.slug, "a");
        assert_eq!(ensuite.reason, Reason::Explicit);

        let enfin = queue.suivante().unwrap();
        assert_eq!(enfin.request.slug, "a");
        assert_eq!(enfin.reason, Reason::Declared { by: "b".into() });
        assert!(queue.est_vide());
    }

    #[test]
    fn remplacer_un_build_oublie_les_dependances_de_celui_qu_on_ecarte() {
        // Sinon on installe les bibliothèques de la version écartée en plus de
        // celles de la version retenue, et le verrou les consigne comme
        // dépendances d'un build qui n'est pas là.
        let mut queue = FileDeResolution::default();
        let x = cle(Origin::Modrinth, "X");
        let y = cle(Origin::Modrinth, "Y");
        queue.pousser(
            Request::new("libA"),
            Reason::Declared { by: "X".into() },
            Some(x.clone()),
        );
        queue.pousser(
            Request::new("libB"),
            Reason::Declared { by: "X".into() },
            Some(x.clone()),
        );
        queue.pousser(
            Request::new("libC"),
            Reason::Declared { by: "Y".into() },
            Some(y),
        );

        queue.oublier_dependances_de(&x);

        // Celles d'un autre demandeur restent : Y n'a pas été remplacé.
        let reste = queue.suivante().unwrap();
        assert_eq!(reste.request.slug, "libC");
        assert!(queue.est_vide());
    }

    #[test]
    fn oublier_les_dependances_epargne_l_homonyme_venu_de_l_autre_source() {
        // « Jade » se publie sous le même titre chez les deux sources, et les
        // deux projets coexistent jusqu'à la déduplication finale. Retirer les
        // dépendances par le titre emportait celles du jumeau, que plus rien ne
        // repoussait : le pack partait sans sa bibliothèque.
        let mut queue = FileDeResolution::default();
        let modrinth = cle(Origin::Modrinth, "nvQzSEkR");
        let curseforge = cle(Origin::CurseForge, "324717");
        queue.pousser(
            Request::new("libA"),
            Reason::Declared { by: "Jade".into() },
            Some(modrinth.clone()),
        );
        queue.pousser(
            Request::new("libB"),
            Reason::Declared { by: "Jade".into() },
            Some(curseforge),
        );

        queue.oublier_dependances_de(&modrinth);

        let reste = queue.suivante().unwrap();
        assert_eq!(reste.request.slug, "libB");
        assert!(queue.est_vide());
    }

    #[test]
    fn oublier_les_dependances_epargne_le_manifeste() {
        // Un mod du manifeste qui porte le nom d'un parent remplacé n'a pas à
        // disparaître : sa demande ne vient pas de ce parent.
        let mut queue = FileDeResolution::default();
        let x = cle(Origin::Modrinth, "X");
        queue.pousser(Request::new("libA"), Reason::Explicit, None);
        queue.pousser(
            Request::new("libA"),
            Reason::Declared { by: "X".into() },
            Some(x.clone()),
        );

        queue.oublier_dependances_de(&x);

        let reste = queue.suivante().unwrap();
        assert_eq!(reste.request.slug, "libA");
        assert_eq!(reste.reason, Reason::Explicit);
        assert!(queue.est_vide());
    }

    #[test]
    fn le_manifeste_fait_autorite_sur_une_dependance_meme_epinglee() {
        let mut epinglee = Request::new("jade");
        epinglee.file = Some("abc".into());

        assert!(
            autorite(&Reason::Explicit, &Request::new("jade"))
                > autorite(&Reason::Declared { by: "x".into() }, &epinglee)
        );
    }

    #[test]
    fn a_origine_egale_l_epinglage_fait_autorite() {
        let mut epinglee = Request::new("jade");
        epinglee.file = Some("abc".into());
        let raison = Reason::Declared { by: "x".into() };

        assert!(autorite(&raison, &epinglee) > autorite(&raison, &Request::new("jade")));

        // Le numéro de version épingle tout autant que l'identifiant de build.
        let mut par_version = Request::new("jade");
        par_version.version = Some("1.2.3".into());
        assert_eq!(
            autorite(&raison, &par_version),
            autorite(&raison, &epinglee)
        );
    }

    #[test]
    fn une_dependance_declaree_fait_autorite_sur_une_dependance_implicite() {
        assert!(
            autorite(&Reason::Declared { by: "x".into() }, &Request::new("lib"))
                > autorite(
                    &Reason::Implicit {
                        by: "x".into(),
                        mod_id: "lib".into()
                    },
                    &Request::new("lib")
                )
        );
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
