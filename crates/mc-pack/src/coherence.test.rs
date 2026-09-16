use super::*;
use crate::lockfile::{LockedLoader, LockedMod};
use mc_mods::Origin;

fn verrouille(slug: &str, side: &str) -> LockedMod {
    LockedMod {
        slug: slug.into(),
        name: slug.into(),
        origin: Origin::Modrinth,
        project: slug.into(),
        file: "1".into(),
        version: "1.0".into(),
        file_name: format!("{slug}.jar"),
        url: format!("https://exemple.invalid/{slug}.jar"),
        sha1: None,
        sha512: None,
        size: 0,
        side: side.into(),
        reason: "demandé par le manifeste".into(),
        provides: vec![slug.into()],
    }
}

#[test]
fn un_mod_du_verrou_absent_de_l_instance_se_voit() {
    // Le cas réel : « install » relancé avec un autre SAMFLIX_ENV a vidé
    // puis regarni l'instance — une seule pour les trois environnements —
    // pendant que le verrou de celui-ci, rangé dans un cache à part, décrit
    // encore les mods d'avant.
    let racine = std::env::temp_dir().join(format!("mc-pack-mods-{}", std::process::id()));
    let layout = mc_instance::Layout::new(racine.clone());
    let instance = layout.instance("samflix");
    std::fs::create_dir_all(instance.mods_dir()).unwrap();
    std::fs::write(instance.mods_dir().join("jei.jar"), b"").unwrap();

    let lock = Lockfile {
        schema: 1,
        pack: "samflix".into(),
        generated: "2025-01-01T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        mods: vec![
            verrouille("jei", "both"),
            verrouille("jade", "client"),
            // Un mod de serveur n'a rien à faire dans l'instance du client :
            // le signaler manquant interdirait tout démarrage.
            verrouille("spark", "server"),
        ],
        unresolved: Vec::new(),
    };

    let manquants = mods_client_absents(&lock, &instance);
    std::fs::remove_dir_all(&racine).ok();
    assert_eq!(manquants, vec!["jade.jar".to_string()]);
}
