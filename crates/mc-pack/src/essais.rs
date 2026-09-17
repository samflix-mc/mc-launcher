//! Un pack sur le disque, et de quoi le servir.
//!
//! Tout ce que fait ce crate part d'un manifeste et d'un verrou : les lire, les
//! écrire, les confronter à ce qui est installé. Les fabriquer à la main dans
//! chaque suite noierait ce que chacune vérifie — un verrou a huit champs et
//! chaque mod en a quatorze.

use std::path::PathBuf;

use crate::lockfile::{LockedLoader, LockedMissing, LockedMod, Lockfile};

/// Un répertoire de travail propre, effacé à la destruction.
pub(crate) struct Atelier {
    pub(crate) racine: PathBuf,
}

impl Atelier {
    pub(crate) fn neuf(nom: &str) -> Atelier {
        let racine = std::env::temp_dir().join(format!(
            "mc-pack-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Atelier { racine }
    }

    pub(crate) fn ecrire(&self, relatif: &str, contenu: &[u8]) -> PathBuf {
        let chemin = self.racine.join(relatif);
        std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
        std::fs::write(&chemin, contenu).unwrap();
        chemin
    }

    pub(crate) fn options(&self) -> crate::Options {
        crate::Options {
            layout: mc_instance::Layout::new(self.racine.join("données")),
            instance_name: Some("samflix".into()),
            ..Default::default()
        }
    }

    /// Écrit le manifeste, son verrou, et ce que `mc_instance::verify` exige.
    ///
    /// Le lancement ne va jamais chercher le pack publié : il ouvre ce qui est
    /// posé sur la machine. Une suite qui le vérifie doit donc poser un pack
    /// complet, pas seulement un manifeste.
    pub(crate) fn pack_installe(&self, mods: Vec<LockedMod>) -> crate::source::Source {
        let manifeste = self.racine.join("samflix.json");
        std::fs::write(&manifeste, MANIFESTE).unwrap();
        verrou(mods.clone())
            .save(&self.racine.join("samflix.lock.json"))
            .unwrap();

        let options = self.options();
        let shared = options.layout.shared();
        for (chemin, contenu) in [
            ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
            ("versions/1.21.1/1.21.1.jar", "jar"),
            (
                "versions/neoforge-21.1.250/neoforge-21.1.250.json",
                r#"{"libraries":[]}"#,
            ),
        ] {
            let cible = shared.join(chemin);
            std::fs::create_dir_all(cible.parent().unwrap()).unwrap();
            std::fs::write(cible, contenu).unwrap();
        }

        let instance = options.layout.instance("samflix");
        std::fs::create_dir_all(instance.mods_dir()).unwrap();
        std::fs::create_dir_all(instance.dir.join("server").join("mods")).unwrap();
        for entree in &mods {
            for dossier in [
                instance.mods_dir(),
                instance.dir.join("server").join("mods"),
            ] {
                std::fs::write(dossier.join(&entree.file_name), JAR).unwrap();
            }
        }

        crate::source::Source::parse(manifeste.to_str().unwrap(), &options.layout)
    }
}

/// Ce qu'on écrit à la place d'un vrai jar.
pub(crate) const JAR: &[u8] = b"le jar";

impl Drop for Atelier {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

/// Le manifeste minimal que le contrôle accepte.
pub(crate) const MANIFESTE: &str = r#"{"schema":1,"name":"samflix","minecraft":"1.21.1",
  "loader":{"type":"neoforge","version":"21.1.250"},
  "servers":{"production":{"host":"mc.ggy.info"},
             "development":{"host":"78.46.100.5","port":25566}}}"#;

/// Un verrou, avec les mods qu'on lui donne.
pub(crate) fn verrou(mods: Vec<LockedMod>) -> Lockfile {
    Lockfile {
        schema: crate::manifest::SCHEMA,
        name: "samflix".into(),
        version: None,
        generated: "2026-09-17T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        servers: Default::default(),
        mods,
        unresolved: Vec::new(),
    }
}

/// Une ligne de verrou : un mod, son côté, son empreinte.
pub(crate) fn entree(slug: &str, side: &str, contenu: Option<&[u8]>) -> LockedMod {
    LockedMod {
        slug: slug.to_string(),
        name: slug.to_string(),
        source: mc_mods::Origin::Modrinth,
        project: format!("{slug}-id"),
        file: format!("{slug}-1.0"),
        version: "1.0".into(),
        channel: mc_mods::Channel::Release,
        file_name: format!("{slug}.jar"),
        url: format!("https://exemple.invalid/{slug}.jar"),
        sha1: contenu.map(|c| mc_dl::Checksum::Sha1(String::new()).of(c)),
        sha512: None,
        size: contenu.map(|c| c.len() as u64).unwrap_or(0),
        side: side.to_string(),
        reason: "demandé".into(),
        provides: vec![slug.to_string()],
    }
}

pub(crate) fn manque(mod_id: &str, exige_par: &str) -> LockedMissing {
    LockedMissing {
        mod_id: mod_id.to_string(),
        required_by: exige_par.to_string(),
        side: "both".into(),
    }
}

/// Sérialise les tests qui posent `SAMFLIX_ENV`.
///
/// L'environnement est lu par tout ce qui choisit un serveur ou journalise une
/// commande : deux tests qui le changent en même temps se contredisent. Verrou
/// atomique et non `Mutex`, pour qu'il traverse aussi les tests asynchrones.
static ENVIRONNEMENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Environnement;

/// Pose l'environnement de déploiement, et le retire en se détruisant.
pub(crate) fn environnement(valeur: &str) -> Environnement {
    use std::sync::atomic::Ordering;
    while ENVIRONNEMENT
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    let garde = Environnement;
    garde.poser(valeur);
    garde
}

impl Environnement {
    pub(crate) fn poser(&self, valeur: &str) {
        // SAFETY : le verrou garantit qu'aucun autre test de ce binaire ne lit
        // ni n'écrit SAMFLIX_ENV tant que le garde vit.
        unsafe {
            std::env::set_var("SAMFLIX_ENV", valeur);
        }
    }
}

impl Drop for Environnement {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("SAMFLIX_ENV");
        }
        ENVIRONNEMENT.store(false, std::sync::atomic::Ordering::Release);
    }
}
