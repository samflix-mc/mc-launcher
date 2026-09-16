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
use std::collections::BTreeMap;
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
    /// Où se connecter, par environnement.
    ///
    /// Le manifeste est le même partout : c'est la même image de contenu,
    /// servie sous trois noms. Ce n'est donc pas lui qui peut choisir, c'est le
    /// client — avec son propre environnement, celui que la CI lui a figé à la
    /// compilation. Un launcher de dev rejoint le serveur de dev.
    ///
    /// Les clés sont celles de `mc_log::Environment` : `development`,
    /// `preproduction`, `production`. Une absence n'est pas une erreur — la
    /// préproduction n'a pas de serveurs Minecraft derrière elle, et le jeu s'y
    /// lance sans rejoindre quoi que ce soit.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub servers: BTreeMap<String, Server>,
}

/// Adresse d'un serveur, telle que `--quickPlayMultiplayer` l'attend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    /// Absent = 25565, le port par défaut de Minecraft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

impl Server {
    /// L'adresse telle que `--quickPlayMultiplayer` la lit.
    ///
    /// Minecraft confie cette chaîne à `HostAndPort` de Guava, qui refuse tout
    /// ce qui porte plus d'un « : » sans crochets. Une IPv6 en porte au moins
    /// deux : écrite nue, elle ne donne pas une mauvaise adresse, elle n'en
    /// donne aucune, et le jeu s'ouvre sur le menu sans rien annoncer. D'où les
    /// crochets, posés ici plutôt qu'attendus de la main qui écrit le manifeste.
    ///
    /// Le port est écrit dès qu'il est déclaré, même quand il vaut 25565.
    /// Omettre celui-là paraît sans conséquence, mais Minecraft ne se contente
    /// pas de composer l'adresse : sans port, il interroge l'enregistrement SRV
    /// « _minecraft._tcp.<hôte> » avant de se rabattre sur le défaut. Savoir si
    /// les deux écritures mènent au même serveur demande de connaître la zone
    /// DNS, que le launcher ne voit pas. On ne jette donc pas ce que le
    /// manifeste a pris la peine de dire.
    pub fn address(&self) -> String {
        let host = self.host.trim();
        let hote = if est_ipv6_nue(host) {
            format!("[{host}]")
        } else {
            host.to_string()
        };
        match self.port {
            Some(port) => format!("{hote}:{port}"),
            None => hote,
        }
    }
}

/// Une IPv6 écrite sans ses crochets.
///
/// Deux « : » au moins : une IPv6 en contient toujours au minimum deux, là où
/// « hôte:port » n'en a qu'un. C'est la distinction que fait Guava, donc celle
/// que fait Minecraft.
fn est_ipv6_nue(host: &str) -> bool {
    !host.starts_with('[') && host.matches(':').count() >= 2
}

/// L'hôte porte-t-il déjà un « :port » ?
///
/// Lui en ajouter un second donnerait « hôte:25565:25566 », que Guava refuse
/// comme elle refuse une IPv6 nue.
fn porte_deja_un_port(host: &str) -> bool {
    match host.rsplit_once(':') {
        Some((avant, apres)) => {
            (avant.ends_with(']') || !avant.contains(':')) && apres.parse::<u16>().is_ok()
        }
        None => false,
    }
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
            expected_sha512: None,
        })
    }
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest> {
        let raw = std::fs::read(path)
            .with_context(|| format!("lecture du manifeste {}", path.display()))?;
        Manifest::parse(&raw).with_context(|| format!("manifeste {} illisible", path.display()))
    }

    /// Lit un manifeste qui n'a pas de chemin — celui d'une réponse HTTP.
    ///
    /// La vérification est la même que pour un fichier : ce qui arrive du
    /// réseau mérite moins de confiance, pas plus.
    pub fn parse(raw: &[u8]) -> Result<Manifest> {
        let manifest: Manifest = serde_json::from_slice(raw)?;
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
        // « name » désigne un répertoire d'instance : il est joint à la racine
        // des données, et « deploy » supprime ensuite tous les .jar qu'il
        // trouve dans le dossier obtenu. Tant que le manifeste venait d'un
        // fichier qu'on édite soi-même, le champ était de confiance ; depuis
        // que le pack distant est la source par défaut, il vient du réseau.
        //
        // Un « .. » remonte, et un chemin absolu fait mieux : PathBuf::join
        // écarte purement et simplement le préfixe. Un hôte de pack compromis
        // ou une faute de frappe obtiendrait alors une suppression de .jar et
        // une écriture de fichiers là où il veut, à chaque installation.
        //
        // On ne refuse que ce qui sort du répertoire. Pas de liste blanche de
        // caractères : refuser ici ce qui est seulement inhabituel
        // condamnerait un pack futur chez tous les launchers déjà distribués,
        // et un launcher qui refuse le pack ne peut plus se dépanner — il
        // faudrait télécharger le pack qu'il refuse de lire.
        let nom = self.name.trim();
        if nom.is_empty() || nom == "." || nom == ".." || nom.contains('/') || nom.contains('\\') {
            bail!(
                "« {} » ne peut pas nommer un répertoire d'instance : le pack \
                 s'installerait hors de la racine des données",
                self.name
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

        // Les clés de « servers » ne sont pas contrôlées ici : voir
        // problemes_de_serveurs, et la raison pour laquelle ce contrôle-là ne
        // peut pas vivre dans check.
        Ok(())
    }

    /// Clés de `servers` que la lecture ne trouvera jamais, chacune avec sa
    /// raison. Vide quand tout est lisible.
    ///
    /// Délibérément hors de `check` : `check` s'applique à tout manifeste, y
    /// compris celui qu'on vient de télécharger. Refuser là un pack pour une
    /// clé inconnue reviendrait à arrêter net tous les launchers déjà
    /// distribués le jour où mc-content déclare un environnement de plus — un
    /// binaire compilé avant ne connaît pas les noms inventés après lui, et il
    /// n'a aucune raison de tenir son ignorance pour une faute du pack. Il sait
    /// ce qu'il sait lire ; le reste, il l'ignore, et c'est la seule réponse qui
    /// laisse le format évoluer sans rappeler les binaires.
    ///
    /// Le contrôle a donc lieu là où le manifeste s'écrit — `mc-pack lock`,
    /// qu'on lance avant de publier — et non là où il se lit.
    pub fn problemes_de_serveurs(&self) -> Vec<String> {
        let mut problemes = Vec::new();
        for (cle, serveur) in &self.servers {
            match mc_log::Environment::parse(cle) {
                // Une faute de frappe ne provoque rien à la lecture : la clé ne
                // correspond à aucun environnement, le jeu s'ouvre sur le menu,
                // et cela ressemble exactement à un pack qui n'aurait rien
                // déclaré. C'est le pire des silences — celui qui ressemble à
                // une intention.
                None => problemes.push(format!(
                    "« {cle} » n'est pas un environnement : attendus development, \
                     preproduction ou production"
                )),
                Some(mc_log::Environment::Local) => problemes.push(format!(
                    "« {cle} » ne serait jamais lu : un binaire compilé à la main \
                     rejoint le serveur de « development »"
                )),
                // Le nom canonique, et lui seul. Environment::parse accepte les
                // alias courants — « dev », « staging » — mais la lecture
                // cherche « development » : l'entrée passerait ici et resterait
                // introuvable là-bas.
                Some(environnement) if environnement.as_str() != cle => problemes.push(format!(
                    "« {cle} » est un alias ; écrire « {} », qui est le nom cherché \
                     à la lecture",
                    environnement.as_str()
                )),
                Some(_) => {}
            }
            let host = serveur.host.trim();
            if host.is_empty() {
                problemes.push(format!("le serveur déclaré pour « {cle} » n'a pas d'hôte"));
            } else if serveur.port.is_some() && porte_deja_un_port(host) {
                problemes.push(format!(
                    "l'hôte de « {cle} » porte déjà un port : « {host} » et le champ \
                     « port » donneraient une adresse à deux ports, que Minecraft refuse"
                ));
            }
        }
        problemes
    }

    pub fn requests(&self) -> Result<Vec<Request>> {
        self.mods.iter().map(ModEntry::to_request).collect()
    }

    /// Version majeure de Java à garantir.
    pub fn java_major(&self, mojang_says: u32) -> u32 {
        self.java.unwrap_or(mojang_says)
    }

    /// Environnement dont les serveurs s'appliquent à celui-ci.
    ///
    /// `local` retombe sur `development` : un binaire compilé à la main est un
    /// binaire de travail, et le serveur de travail est celui de dev. C'est le
    /// même raisonnement que pour l'adresse du pack, et il vaut mieux qu'ils ne
    /// divergent pas — un launcher qui installerait le pack de dev pour
    /// rejoindre la production ferait exactement ce que tout ceci empêche.
    ///
    /// Rendu public parce que l'affichage en a besoin : dire « local » à
    /// quelqu'un qui diagnostique, alors que l'entrée lue est « development »,
    /// l'envoie chercher une clé qui n'existe pas.
    pub fn environnement_serveur(env: mc_log::Environment) -> mc_log::Environment {
        match env {
            mc_log::Environment::Local => mc_log::Environment::Development,
            autre => autre,
        }
    }

    /// Serveur à rejoindre pour un environnement donné.
    pub fn server_for(&self, env: mc_log::Environment) -> Option<&Server> {
        self.servers.get(Self::environnement_serveur(env).as_str())
    }
}

#[cfg(test)]
#[path = "manifest.test.rs"]
mod tests;
