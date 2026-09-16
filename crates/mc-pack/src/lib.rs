//! Enchaînement complet : d'un manifeste JSON à une instance jouable.
//!
//! L'ordre des étapes n'est pas arbitraire, chacune dépend de la précédente :
//!
//! 1. **le chargeur** — `latest` est résolu tout de suite, pour que le verrou
//!    consigne une version exacte et non un mot ;
//! 2. **les fichiers de Mojang** — ils donnent au passage la version de Java
//!    qu'exige cette version du jeu ;
//! 3. **Java** — détecté ou installé, en s'appuyant sur ce que Mojang exige ;
//! 4. **NeoForge** — son installateur patche le client vanilla et a besoin du
//!    Java de l'étape précédente ;
//! 5. **les mods** — résolus, téléchargés, puis répartis entre client et
//!    serveur ;
//! 6. **le verrou** — écrit en dernier, il décrit ce qui a réellement été fait.

pub mod lockfile;
pub mod manifest;
pub mod source;

use anyhow::{Context, Result};
use lockfile::{LockedLoader, Lockfile};
use mc_mods::Side;
use source::{Pack, Source};
use std::path::PathBuf;

/// Ce qu'une installation a produit, pour le compte rendu.
#[derive(Debug)]
pub struct Outcome {
    pub instance: mc_instance::Instance,
    pub server_dir: PathBuf,
    pub java: mc_java::Java,
    pub neoforge: String,
    pub assets_downloaded: usize,
    pub libraries: usize,
    pub client_mods: usize,
    pub server_mods: usize,
    pub removed: Vec<String>,
    pub lock: Lockfile,
    pub lock_path: PathBuf,
    pub previous_lock: Option<Lockfile>,
    /// D'où venait le pack, tel qu'on l'a demandé.
    pub source: String,
    /// Le pack distant était injoignable et la copie locale a servi.
    pub from_cache: bool,
}

#[derive(Default)]
pub struct Options {
    /// Rejouer exactement le verrou au lieu de rechercher les versions.
    pub locked: bool,
    /// Installer aussi un serveur NeoForge complet, et pas seulement ses mods.
    pub with_server: bool,
    /// Nom de l'instance ; par défaut, celui du pack.
    pub instance_name: Option<String>,
    pub layout: mc_instance::Layout,
}

/// Journal des étapes, pour que l'appelant décide de l'affichage.
pub type Progress<'a> = &'a (dyn Fn(&str) + Sync);

#[tracing::instrument(
    name = "installation",
    skip(options, log),
    fields(pack, minecraft, source = %source.describe(), rejeu, serveur = options.with_server)
)]
pub async fn install(source: &Source, options: &Options, log: Progress<'_>) -> Result<Outcome> {
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT)?;
    let Pack {
        manifest,
        lock: previous_lock,
        lock_path,
        replay,
        from_cache,
    } = source.load(&dl).await?;

    // Un pack distant se rejoue toujours : c'est le verrou publié qui décide
    // des versions, pas la machine du joueur. Voir `source`.
    let replay = replay || options.locked;

    // Renseignés après lecture du manifeste : le span les porte, donc tout ce
    // qui suit est rattaché au pack sans avoir à le répéter à chaque ligne.
    tracing::Span::current().record("pack", &manifest.name);
    tracing::Span::current().record("minecraft", &manifest.minecraft);
    tracing::Span::current().record("rejeu", replay);

    if from_cache {
        log("Hors-ligne : pack repris de la dernière copie connue.");
    }

    let layout = &options.layout;
    let shared = layout.shared();

    // --- 1. Chargeur ---------------------------------------------------------
    let neoforge_version = if replay {
        let lock = previous_lock
            .as_ref()
            .with_context(|| format!("{} absent : rien à rejouer", lock_path.display()))?;
        lock.loader.version.clone()
    } else if manifest.loader.is_latest() {
        mc_instance::neoforge::latest_for(&manifest.minecraft, &dl).await?
    } else {
        manifest.loader.version.clone()
    };
    tracing::info!(
        minecraft = %manifest.minecraft,
        neoforge = %neoforge_version,
        epingle = !manifest.loader.is_latest(),
        "Minecraft {} avec NeoForge {neoforge_version}",
        manifest.minecraft
    );
    log(&format!(
        "Minecraft {} — NeoForge {neoforge_version}",
        manifest.minecraft
    ));

    // --- 2. Fichiers de Mojang ----------------------------------------------
    log("Fichiers du jeu…");
    let game = mc_instance::vanilla::install(&manifest.minecraft, &shared, &dl)
        .await
        .with_context(|| format!("installation de Minecraft {}", manifest.minecraft))?;
    tracing::info!(
        bibliotheques = game.libraries.len(),
        assets_telecharges = game.assets_downloaded,
        index_assets = %game.asset_index_id,
        "Fichiers du jeu en place : {} bibliothèques, {} assets téléchargés",
        game.libraries.len(),
        game.assets_downloaded
    );
    log(&format!(
        "  {} bibliothèques, {} assets téléchargés",
        game.libraries.len(),
        game.assets_downloaded
    ));

    // --- 3. Java -------------------------------------------------------------
    let java_major = manifest.java_major(game.java_major);
    let java = mc_java::ensure(java_major, &layout.runtime())
        .await
        .with_context(|| format!("aucun Java {java_major} utilisable"))?;
    tracing::info!(
        version = %java.version.full,
        majeur_exige = java_major,
        origine = ?java.origin,
        "Java {} utilisé ({})",
        java.version.full,
        match java.origin {
            mc_java::Origin::Managed => "installé par le launcher",
            mc_java::Origin::System => "runtime du système",
        }
    );
    log(&format!(
        "Java {} — {}",
        java.version.full,
        java.path.display()
    ));

    // --- 4. NeoForge ---------------------------------------------------------
    log("Chargeur NeoForge…");
    mc_instance::neoforge::install_client(
        &neoforge_version,
        &shared,
        &layout.cache(),
        &java.path,
        &dl,
    )
    .await
    .with_context(|| format!("installation de NeoForge {neoforge_version}"))?;
    tracing::info!(
        version = %neoforge_version,
        "Chargeur NeoForge {neoforge_version} en place"
    );

    // --- 5. Mods -------------------------------------------------------------
    let registry = mc_mods::Registry::new(layout.cache().join("mods"))?;
    let requests = if replay {
        let lock = previous_lock.as_ref().expect("vérifié plus haut");
        tracing::info!(
            builds = lock.mods.len(),
            "Rejeu du verrou : {} builds épinglés",
            lock.mods.len()
        );
        log(&format!(
            "Mods : {} builds rejoués depuis le verrou",
            lock.mods.len()
        ));
        lock.requests()
    } else {
        tracing::info!(
            demandes = manifest.mods.len(),
            "Résolution de {} mods demandés",
            manifest.mods.len()
        );
        log("Résolution des mods…");
        manifest.requests()?
    };

    let plan = mc_mods::resolve(&registry, &requests, &manifest.minecraft, "neoforge").await?;
    let added = plan
        .mods
        .iter()
        .filter(|m| m.reason != mc_mods::Reason::Explicit)
        .count();
    tracing::info!(
        total = plan.mods.len(),
        ajoutes = added,
        non_resolus = plan.unresolved.len(),
        "{} mods résolus, dont {added} ajoutés par dépendance",
        plan.mods.len()
    );
    log(&format!(
        "  {} mods, dont {added} ajoutés par résolution des dépendances",
        plan.mods.len()
    ));
    for entry in plan
        .mods
        .iter()
        .filter(|m| matches!(m.reason, mc_mods::Reason::Implicit { .. }))
    {
        log(&format!(
            "  · {} — {}",
            entry.candidate.slug,
            entry.reason.describe()
        ));
    }

    let instance = layout.instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    instance.create()?;
    let server_dir = instance.dir.join("server");

    let client = mc_mods::resolve::deploy(&plan, Side::Client, &instance.mods_dir())?;
    let server = mc_mods::resolve::deploy(&plan, Side::Server, &server_dir.join("mods"))?;
    tracing::info!(
        instance = %instance.name,
        client = client.installed,
        serveur = server.installed,
        retires = client.removed.len() + server.removed.len(),
        "Instance « {} » : {} mods côté client, {} côté serveur",
        instance.name,
        client.installed,
        server.installed
    );

    if options.with_server {
        log("Serveur NeoForge…");
        mc_instance::neoforge::install_server(
            &neoforge_version,
            &server_dir,
            &layout.cache(),
            &java.path,
            &dl,
        )
        .await
        .with_context(|| format!("installation du serveur NeoForge {neoforge_version}"))?;
        tracing::info!(
            repertoire = %server_dir.display(),
            "Serveur NeoForge installé dans {}",
            server_dir.display()
        );
    }

    // --- 6. Verrou -----------------------------------------------------------
    //
    // Rejouer un verrou, c'est lui obéir, pas le réécrire. Le régénérer
    // effacerait la colonne `reason` : tout y deviendrait « demandé par le
    // manifeste », puisque c'est le verrou lui-même qui a dicté les demandes,
    // et on perdrait la seule trace de ce qui n'avait jamais été demandé.
    let lock = match (replay, &previous_lock) {
        (true, Some(existing)) => {
            tracing::debug!(
                verrou = %lock_path.display(),
                "verrou rejoué, laissé tel quel"
            );
            existing.clone()
        }
        _ => {
            let fresh = Lockfile::from_plan(
                &manifest.name,
                &manifest.minecraft,
                LockedLoader {
                    kind: manifest.loader.kind.clone(),
                    version: neoforge_version.clone(),
                },
                java_major,
                &plan,
            );
            fresh.save(&lock_path)?;
            fresh
        }
    };

    let mut removed = client.removed;
    removed.extend(server.removed);

    Ok(Outcome {
        instance,
        server_dir,
        java,
        neoforge: neoforge_version,
        assets_downloaded: game.assets_downloaded,
        libraries: game.libraries.len(),
        client_mods: client.installed,
        server_mods: server.installed,
        removed,
        lock,
        lock_path,
        previous_lock,
        source: source.describe(),
        from_cache,
    })
}

/// Contrôle une installation existante à partir du manifeste et du verrou.
///
/// Trois questions, dans l'ordre où elles font échouer un démarrage : les
/// fichiers du jeu sont-ils là, les jars annoncés par le verrou sont-ils
/// présents et intacts, et chaque `modId` exigé est-il fourni ?
pub fn verify(source: &Source, options: &Options, deep: bool) -> Result<Vec<String>> {
    let pack = source.load_local()?;
    let manifest = pack.manifest;
    let lock = pack.lock.with_context(|| {
        format!(
            "{} absent : rien à vérifier tant que « mc-pack install » n'a pas tourné",
            pack.lock_path.display()
        )
    })?;

    let mut problems = mc_instance::verify(
        &manifest.minecraft,
        &lock.loader.version,
        &options.layout,
        deep,
    )?;

    let instance = options
        .layout
        .instance(options.instance_name.as_deref().unwrap_or(&manifest.name));
    let server_mods = instance.dir.join("server").join("mods");

    for entry in &lock.mods {
        let side = Side::parse(&entry.side).unwrap_or(Side::Both);
        let mut targets = Vec::new();
        if side.includes(Side::Client) {
            targets.push(instance.mods_dir().join(&entry.file_name));
        }
        if side.includes(Side::Server) {
            targets.push(server_mods.join(&entry.file_name));
        }

        for path in targets {
            if !path.is_file() {
                problems.push(format!("mod manquant : {}", path.display()));
                continue;
            }
            let Some(expected) = &entry.sha1 else {
                continue;
            };
            match mc_dl::sha1_of_file(&path) {
                Ok(got) if got.eq_ignore_ascii_case(expected) => {}
                Ok(got) => problems.push(format!(
                    "{} : empreinte {got} au lieu de {expected}",
                    path.display()
                )),
                Err(e) => problems.push(format!("{} : illisible ({e})", path.display())),
            }
        }
    }

    // Le verrou porte les `modId` fournis par chaque jar : la cohérence de
    // l'ensemble se vérifie sans rouvrir une seule archive.
    let provided: std::collections::BTreeSet<&String> =
        lock.mods.iter().flat_map(|m| m.provides.iter()).collect();
    for missing in &lock.unresolved {
        if !provided.contains(&missing.mod_id) {
            problems.push(format!(
                "dépendance non satisfaite : {} exigé par {}",
                missing.mod_id, missing.required_by
            ));
        }
    }

    Ok(problems)
}
