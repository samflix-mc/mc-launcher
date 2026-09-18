//! Une installation complète sur le disque, côté binaire.
//!
//! Les commandes lisent un pack, un verrou et une instance : les fabriquer
//! demande les trois à la fois, et chaque suite les redemanderait autrement.

use std::path::PathBuf;

use mc_pack::lockfile::{LockedLoader, LockedMod, Lockfile};

pub(super) struct Atelier {
    pub(super) racine: PathBuf,
}

impl Atelier {
    pub(super) fn neuf(nom: &str) -> Atelier {
        let racine = std::env::temp_dir().join(format!(
            "mc-pack-bin-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Atelier { racine }
    }

    pub(super) fn options(&self) -> mc_pack::Options {
        mc_pack::Options {
            layout: mc_instance::Layout::new(self.racine.join("données")),
            instance_name: Some("samflix".into()),
            ..Default::default()
        }
    }

    /// Écrit le manifeste, son verrou, et ce que `mc_instance::verify` exige.
    pub(super) fn pack_installe(&self, mods: Vec<LockedMod>) -> mc_pack::source::Source {
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

        mc_pack::source::Source::parse(manifeste.to_str().unwrap(), &options.layout)
    }
}

impl Drop for Atelier {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

pub(super) const JAR: &[u8] = b"le jar";

pub(super) const MANIFESTE: &str = r#"{"schema":1,"name":"samflix","minecraft":"1.21.1",
  "loader":{"type":"neoforge","version":"21.1.250"},
  "servers":{"production":{"host":"mc.ggy.info"},
             "development":{"host":"78.46.100.5","port":25566}}}"#;

pub(super) fn verrou(mods: Vec<LockedMod>) -> Lockfile {
    Lockfile {
        schema: 1,
        name: "samflix".into(),
        version: None,
        generated: "2026-09-17T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        generation: 0,
        servers: Default::default(),
        mods,
        unresolved: Vec::new(),
    }
}

pub(super) fn entree(slug: &str, side: &str) -> LockedMod {
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
        sha1: Some(mc_dl::Checksum::Sha1(String::new()).of(JAR)),
        sha512: None,
        size: JAR.len() as u64,
        side: side.to_string(),
        reason: "demandé".into(),
        provides: vec![slug.to_string()],
    }
}

/// Sérialise les tests qui posent `SAMFLIX_ENV`.
///
/// L'environnement est lu par tout ce qui choisit un serveur ou journalise une
/// commande : deux tests qui le changent en même temps se contredisent. Verrou
/// atomique et non `Mutex`, pour qu'il traverse aussi les tests asynchrones.
static ENVIRONNEMENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) struct Environnement;

/// Pose l'environnement de déploiement, et le retire en se détruisant.
pub(super) fn environnement(valeur: &str) -> Environnement {
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
    pub(super) fn poser(&self, valeur: &str) {
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
