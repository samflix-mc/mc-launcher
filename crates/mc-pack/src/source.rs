//! D'où vient le pack : un fichier du dépôt, ou une adresse.
//!
//! Les deux cas ne servent pas les mêmes gens, et c'est ce qui décide de leur
//! comportement.
//!
//! **Un fichier** est ce qu'on édite. Le manifeste dit ce qu'on veut, la
//! résolution cherche les versions, et le verrou est réécrit à côté. C'est le
//! geste de développement, celui qui fait bouger le pack.
//!
//! **Une adresse** est ce qu'on reçoit. Le manifeste et son verrou sont
//! téléchargés ensemble, et le verrou est **rejoué tel quel** : un joueur ne
//! résout rien. S'il le faisait, sa machine choisirait ses propres versions le
//! jour où un mod en publie une nouvelle, et il arriverait sur le serveur avec
//! des registres NeoForge qui ne concordent plus — une éjection à la connexion,
//! sans message utile.
//!
//! Le verrou distant est donc la seule source de vérité côté joueur, et c'est
//! exactement ce que mc-content publie.
//!
//! ## Hors-ligne
//!
//! Chaque téléchargement réussi laisse une copie dans le cache. Quand le réseau
//! manque, cette copie est reprise et l'utilisateur en est averti : jouer avec
//! le pack d'hier vaut mieux que ne pas jouer. Rien n'est mis en cache avant
//! d'avoir été relu — une réponse tronquée ou une page d'erreur HTML
//! remplaceraient sinon un pack valide par du vide.

use crate::lockfile::Lockfile;
use crate::manifest::Manifest;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Adresse par défaut du pack : mc-launcher-site sert le répertoire `launcher/`
/// de l'image mc-content, qui est l'endroit où la liste des mods est décidée.
pub const DEFAULT_URL: &str = "https://mc-launcher.ggy.info/pack/samflix.json";

#[derive(Debug, Clone)]
pub enum Source {
    /// Un chemin sur le disque. Le verrou se trouve à côté, et sera réécrit.
    File { manifest: PathBuf },
    /// Une URL. Le manifeste et le verrou sont téléchargés puis mis en cache.
    Remote { url: String, cache_dir: PathBuf },
}

/// Un pack lu, quelle qu'en soit la provenance.
pub struct Pack {
    pub manifest: Manifest,
    /// Verrou déjà connu : celui du dépôt, ou celui qui vient d'être
    /// téléchargé. Absent la première fois qu'un pack local est résolu.
    pub lock: Option<Lockfile>,
    /// Où écrire le verrou, quand il y a lieu de l'écrire.
    pub lock_path: PathBuf,
    /// Le verrou fait foi : ses builds sont rejoués au lieu d'être cherchés.
    pub replay: bool,
    /// Le réseau a manqué et le cache a pris le relais.
    pub from_cache: bool,
}

impl Source {
    /// Une adresse se reconnaît à son protocole ; tout le reste est un chemin.
    ///
    /// Sans cette règle, installer depuis le dépôt et installer depuis le site
    /// demanderaient deux commandes, alors que c'est le même geste sur le même
    /// objet.
    pub fn parse(arg: &str, layout: &mc_instance::Layout) -> Source {
        if is_url(arg) {
            Source::Remote {
                url: arg.to_string(),
                cache_dir: cache_dir_for(arg, layout),
            }
        } else {
            Source::File {
                manifest: PathBuf::from(arg),
            }
        }
    }

    /// Ce qu'on affiche pour dire d'où vient le pack.
    pub fn describe(&self) -> String {
        match self {
            Source::File { manifest } => manifest.display().to_string(),
            Source::Remote { url, .. } => url.clone(),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Source::Remote { .. })
    }

    pub async fn load(&self, dl: &mc_dl::Downloader) -> Result<Pack> {
        match self {
            Source::File { manifest } => {
                let lock_path = Lockfile::path_for(manifest);
                let lock = lock_path
                    .is_file()
                    .then(|| Lockfile::load(&lock_path))
                    .transpose()?;
                Ok(Pack {
                    manifest: Manifest::load(manifest)?,
                    lock,
                    lock_path,
                    replay: false,
                    from_cache: false,
                })
            }
            Source::Remote { url, cache_dir } => load_remote(url, cache_dir, dl).await,
        }
    }

    /// Le pack tel qu'il est posé sur cette machine, sans toucher au réseau.
    ///
    /// C'est ce que lisent `launch` et `verify`, et pour la même raison : tous
    /// deux parlent de l'installation présente sur le disque. Aller rechercher
    /// le pack publié les ferait décrire un pack qui n'est pas celui qu'on a
    /// installé — et lancer une partie cesserait de marcher sans réseau, ce
    /// qui est précisément le moment où l'on veut jouer.
    ///
    /// C'est `install` qui rafraîchit la copie locale, et lui seul.
    pub fn load_local(&self) -> Result<Pack> {
        let manifest_path = match self {
            Source::File { manifest } => manifest.clone(),
            Source::Remote { url, cache_dir } => {
                let cached = cache_dir.join(file_name_of(url));
                if !cached.is_file() {
                    bail!("pack {url} jamais installé — lancer « mc-pack install » d'abord");
                }
                cached
            }
        };
        let lock_path = Lockfile::path_for(&manifest_path);
        let lock = lock_path
            .is_file()
            .then(|| Lockfile::load(&lock_path))
            .transpose()?;
        Ok(Pack {
            manifest: Manifest::load(&manifest_path)?,
            lock,
            lock_path,
            replay: self.is_remote(),
            from_cache: false,
        })
    }

    /// Le chemin du manifeste, quand il y en a un à éditer.
    ///
    /// `lock` en a besoin : résoudre un pack distant n'aurait nulle part où
    /// écrire son résultat.
    pub fn local_path(&self) -> Option<&Path> {
        match self {
            Source::File { manifest } => Some(manifest),
            Source::Remote { .. } => None,
        }
    }
}

async fn load_remote(url: &str, cache_dir: &Path, dl: &mc_dl::Downloader) -> Result<Pack> {
    let lock_url = lock_url_for(url);
    let manifest_cache = cache_dir.join(file_name_of(url));
    let lock_cache = Lockfile::path_for(&manifest_cache);

    // Le manifeste et le verrou sont récupérés ensemble ou pas du tout : un
    // manifeste neuf accompagné du verrou d'avant décrirait un pack que
    // personne n'a jamais publié.
    let fetched = match fetch_pair(dl, url, &lock_url).await {
        Ok(pair) => Some(pair),
        Err(error) => {
            tracing::warn!(
                url,
                erreur = %error,
                "Pack distant injoignable, repli sur la dernière copie connue : {error}"
            );
            None
        }
    };

    if let Some((manifest, lock)) = fetched {
        std::fs::create_dir_all(cache_dir)
            .with_context(|| format!("création du cache {}", cache_dir.display()))?;
        manifest.save(&manifest_cache)?;
        lock.save(&lock_cache)?;
        tracing::info!(
            pack = %manifest.name,
            mods = lock.mods.len(),
            "Pack « {} » repris depuis {url} — {} builds épinglés",
            manifest.name,
            lock.mods.len()
        );
        return Ok(Pack {
            manifest,
            lock: Some(lock),
            lock_path: lock_cache,
            replay: true,
            from_cache: false,
        });
    }

    if !manifest_cache.is_file() || !lock_cache.is_file() {
        bail!(
            "pack {url} injoignable, et aucune copie dans {}",
            cache_dir.display()
        );
    }
    let manifest = Manifest::load(&manifest_cache)?;
    let lock = Lockfile::load(&lock_cache)?;
    tracing::warn!(
        pack = %manifest.name,
        genere = %lock.generated,
        "Hors-ligne : pack « {} » repris du cache, figé au {}",
        manifest.name,
        lock.generated
    );
    Ok(Pack {
        manifest,
        lock: Some(lock),
        lock_path: lock_cache,
        replay: true,
        from_cache: true,
    })
}

async fn fetch_pair(
    dl: &mc_dl::Downloader,
    manifest_url: &str,
    lock_url: &str,
) -> Result<(Manifest, Lockfile)> {
    let raw = dl.bytes(manifest_url).await?;
    let manifest = Manifest::parse(&raw)
        .with_context(|| format!("{manifest_url} ne contient pas un manifeste lisible"))?;
    let raw = dl.bytes(lock_url).await?;
    let lock = Lockfile::parse(&raw)
        .with_context(|| format!("{lock_url} ne contient pas un verrou lisible"))?;
    Ok((manifest, lock))
}

fn is_url(arg: &str) -> bool {
    let lowered = arg.to_ascii_lowercase();
    lowered.starts_with("https://") || lowered.starts_with("http://")
}

/// `…/samflix.json` donne `…/samflix.lock.json`, comme sur le disque.
fn lock_url_for(url: &str) -> String {
    match url.strip_suffix(".json") {
        Some(base) => format!("{base}.lock.json"),
        None => format!("{url}.lock.json"),
    }
}

fn file_name_of(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    match path.rsplit('/').next() {
        Some(name) if name.ends_with(".json") => name.to_string(),
        _ => "pack.json".to_string(),
    }
}

/// Le cache est rangé par hôte, et non à plat.
///
/// Les manifestes de dev et de production portent le même nom de fichier ; à
/// plat, passer de l'un à l'autre écraserait silencieusement le premier, et une
/// panne de réseau ressortirait le pack du mauvais environnement.
fn cache_dir_for(url: &str, layout: &mc_instance::Layout) -> PathBuf {
    let host = url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .filter(|h| !h.is_empty())
        .unwrap_or("inconnu");
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    layout.cache().join("packs").join(safe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_adresse_se_reconnait_a_son_protocole() {
        assert!(is_url("https://mc-launcher.ggy.info/pack/samflix.json"));
        assert!(is_url("HTTP://exemple.invalid/x.json"));
        assert!(!is_url("packs/samflix.json"));
        assert!(!is_url("/var/tmp/samflix.json"));
    }

    #[test]
    fn le_verrou_distant_se_deduit_du_manifeste() {
        assert_eq!(
            lock_url_for("https://exemple.invalid/pack/samflix.json"),
            "https://exemple.invalid/pack/samflix.lock.json"
        );
    }

    #[test]
    fn le_nom_de_fichier_ignore_la_requete() {
        assert_eq!(
            file_name_of("https://exemple.invalid/pack/samflix.json?v=3"),
            "samflix.json"
        );
        assert_eq!(file_name_of("https://exemple.invalid/pack/"), "pack.json");
    }

    #[test]
    fn deux_environnements_ne_partagent_pas_leur_cache() {
        let layout = mc_instance::Layout::new(PathBuf::from("/data"));
        let prod = cache_dir_for("https://mc-launcher.ggy.info/pack/samflix.json", &layout);
        let dev = cache_dir_for(
            "https://mc-launcher-dev.ggy.info/pack/samflix.json",
            &layout,
        );
        assert_ne!(prod, dev);
        assert!(prod.ends_with("mc-launcher.ggy.info"));
    }

    #[test]
    fn un_chemin_reste_un_chemin() {
        let layout = mc_instance::Layout::new(PathBuf::from("/data"));
        let source = Source::parse("packs/samflix.json", &layout);
        assert!(!source.is_remote());
        assert_eq!(source.describe(), "packs/samflix.json");
    }
}
