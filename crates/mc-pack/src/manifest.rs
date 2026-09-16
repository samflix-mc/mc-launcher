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
    /// `hôte:port`, ou `hôte` seul quand le port est celui par défaut.
    pub fn address(&self) -> String {
        match self.port {
            Some(port) if port != 25565 => format!("{}:{}", self.host, port),
            _ => self.host.clone(),
        }
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
            if serveur.host.trim().is_empty() {
                problemes.push(format!("le serveur déclaré pour « {cle} » n'a pas d'hôte"));
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
            servers: BTreeMap::new(),
        }
    }

    fn avec_serveurs() -> Manifest {
        let mut manifest = base();
        manifest.servers.insert(
            "development".into(),
            Server {
                host: "78.46.100.5".into(),
                port: Some(25566),
            },
        );
        manifest.servers.insert(
            "production".into(),
            Server {
                host: "mc.ggy.info".into(),
                port: Some(25565),
            },
        );
        manifest
    }

    #[test]
    fn le_serveur_suit_l_environnement() {
        let manifest = avec_serveurs();
        assert_eq!(
            manifest
                .server_for(mc_log::Environment::Development)
                .map(Server::address),
            Some("78.46.100.5:25566".into())
        );
        assert_eq!(
            manifest
                .server_for(mc_log::Environment::Production)
                .map(Server::address),
            Some("mc.ggy.info".into()),
            "le port par défaut ne s'écrit pas : Minecraft le sous-entend"
        );
    }

    #[test]
    fn un_binaire_local_rejoint_la_dev() {
        // Un binaire compilé à la main est un binaire de travail. Le faire
        // tomber sur la production reviendrait à envoyer quelqu'un qui essaie
        // là où d'autres jouent.
        let manifest = avec_serveurs();
        assert_eq!(
            manifest
                .server_for(mc_log::Environment::Local)
                .map(Server::address),
            manifest
                .server_for(mc_log::Environment::Development)
                .map(Server::address),
        );
    }

    #[test]
    fn une_preproduction_sans_serveur_ne_lance_rien() {
        // Elle n'a pas de serveurs Minecraft derrière elle, et ce n'est pas un
        // oubli : le nœud est unique, chaque réseau complet coûte de la RAM.
        assert!(
            avec_serveurs()
                .server_for(mc_log::Environment::Preproduction)
                .is_none()
        );
    }

    fn avec_cle(cle: &str, host: &str) -> Manifest {
        let mut manifest = base();
        manifest.servers.insert(
            cle.into(),
            Server {
                host: host.into(),
                port: None,
            },
        );
        manifest
    }

    #[test]
    fn une_cle_de_serveur_fautive_est_signalee() {
        // Sans ce contrôle, une faute de frappe ne provoque rien : la clé ne
        // correspond à aucun environnement, le jeu s'ouvre sur le menu, et
        // cela ressemble exactement à un pack qui n'aurait rien déclaré.
        let problemes = avec_cle("prodution", "mc.ggy.info").problemes_de_serveurs();
        assert_eq!(problemes.len(), 1);
        assert!(problemes[0].contains("prodution"));
    }

    #[test]
    fn un_alias_est_signale_parce_qu_il_ne_serait_pas_lu() {
        // Environment::parse accepte « dev », mais server_for cherche
        // « development » : l'entrée passerait ici et resterait introuvable.
        let problemes = avec_cle("dev", "mc-dev.ggy.info").problemes_de_serveurs();
        assert_eq!(problemes.len(), 1);
        assert!(problemes[0].contains("development"));
    }

    #[test]
    fn une_cle_local_est_signalee() {
        let problemes = avec_cle("local", "mc-dev.ggy.info").problemes_de_serveurs();
        assert_eq!(problemes.len(), 1);
        assert!(problemes[0].contains("development"));
    }

    #[test]
    fn un_hote_vide_est_signale() {
        assert_eq!(
            avec_cle("production", "   ").problemes_de_serveurs().len(),
            1
        );
    }

    #[test]
    fn les_cles_canoniques_ne_posent_aucun_probleme() {
        assert!(avec_serveurs().problemes_de_serveurs().is_empty());
    }

    #[test]
    fn un_pack_plus_recent_que_le_binaire_reste_installable() {
        // Le jour où mc-content déclare un environnement de plus, les binaires
        // déjà chez les joueurs ne le connaîtront pas. S'ils refusaient le
        // manifeste pour autant, une ligne ajoutée au pack couperait
        // l'installation de tout le parc d'un coup — et personne ne pourrait
        // plus rien télécharger pour se réparer.
        let brut = br#"{"schema":1,"name":"essai","minecraft":"1.21.1",
                        "loader":{"type":"neoforge","version":"latest"},
                        "servers":{"production":{"host":"mc.ggy.info"},
                                   "qualification":{"host":"mc-qa.ggy.info"}},
                        "nouveau_champ_inconnu":true}"#;
        let manifest = Manifest::parse(brut).expect("un environnement inconnu n'est pas une faute");
        assert_eq!(
            manifest
                .server_for(mc_log::Environment::Production)
                .map(Server::address),
            Some("mc.ggy.info".into()),
            "ce que ce binaire sait lire reste lisible"
        );
        assert!(
            manifest
                .server_for(mc_log::Environment::Development)
                .is_none()
        );
    }

    #[test]
    fn les_serveurs_survivent_a_un_passage_par_le_cache() {
        // launch lit le manifeste rangé dans le cache, pas celui du réseau :
        // un champ perdu à l'écriture ferait s'ouvrir le jeu sur le menu au
        // lieu de rejoindre le serveur, et seulement hors ligne.
        let dossier = std::env::temp_dir().join(format!("mc-pack-essai-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("samflix.json");
        avec_serveurs().save(&chemin).unwrap();
        let relu = Manifest::load(&chemin).unwrap();
        std::fs::remove_dir_all(&dossier).ok();
        assert_eq!(
            relu.server_for(mc_log::Environment::Development)
                .map(Server::address),
            Some("78.46.100.5:25566".into())
        );
    }

    #[test]
    fn un_binaire_local_lit_la_cle_development() {
        assert_eq!(
            Manifest::environnement_serveur(mc_log::Environment::Local),
            mc_log::Environment::Development
        );
        assert_eq!(
            Manifest::environnement_serveur(mc_log::Environment::Production),
            mc_log::Environment::Production
        );
    }

    #[test]
    fn un_manifeste_sans_serveurs_reste_lisible() {
        // Le champ est arrivé après les premiers packs : les manifestes qui
        // l'ignorent doivent continuer de se lire tels quels.
        let brut = br#"{"schema":1,"name":"essai","minecraft":"1.21.1",
                        "loader":{"type":"neoforge","version":"latest"}}"#;
        let manifest = Manifest::parse(brut).expect("manifeste sans serveurs");
        assert!(manifest.servers.is_empty());
        assert!(
            manifest
                .server_for(mc_log::Environment::Production)
                .is_none()
        );
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
