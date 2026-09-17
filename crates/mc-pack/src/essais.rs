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
}

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
        pack: "samflix".into(),
        generated: "2026-09-17T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        mods,
        unresolved: Vec::new(),
    }
}

/// Une ligne de verrou : un mod, son côté, son empreinte.
pub(crate) fn entree(slug: &str, side: &str, contenu: Option<&[u8]>) -> LockedMod {
    LockedMod {
        slug: slug.to_string(),
        name: slug.to_string(),
        origin: mc_mods::Origin::Modrinth,
        project: format!("{slug}-id"),
        file: format!("{slug}-1.0"),
        version: "1.0".into(),
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
