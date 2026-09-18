use super::{Lockfile, missing_client_mods};
use crate::lockfile::{LockedLoader, LockedMod};
use mc_mods::Origin;

fn locked(slug: &str, side: &str) -> LockedMod {
    LockedMod {
        slug: slug.into(),
        name: slug.into(),
        source: Origin::Modrinth,
        project: slug.into(),
        file: "1".into(),
        version: "1.0".into(),
        channel: mc_mods::Channel::Release,
        file_name: format!("{slug}.jar"),
        url: format!("https://exemple.invalid/{slug}.jar"),
        sha1: None,
        sha512: None,
        size: 0,
        side: side.into(),
        reason: "requested by the manifest".into(),
        provides: vec![slug.into()],
    }
}

#[test]
fn a_lockfile_mod_missing_from_the_instance_shows_up() {
    // The real case: `install` rerun with a different SAMFLIX_ENV emptied
    // then refilled the instance — a single one for all three environments —
    // while this one's lockfile, stored in a separate cache, still describes
    // the earlier mods.
    let root = std::env::temp_dir().join(format!("mc-pack-mods-{}", std::process::id()));
    let layout = mc_instance::Layout::new(root.clone());
    let instance = layout.instance("samflix");
    std::fs::create_dir_all(instance.mods_dir()).unwrap();
    std::fs::write(instance.mods_dir().join("jei.jar"), b"").unwrap();

    let lock = Lockfile {
        schema: 1,
        name: "samflix".into(),
        version: None,
        generated: "2025-01-01T00:00:00Z".into(),
        minecraft: "1.21.1".into(),
        loader: LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        java: 21,
        generation: 0,
        servers: Default::default(),
        mods: vec![
            locked("jei", "both"),
            locked("jade", "client"),
            // A server mod has no business in the client instance: flagging
            // it missing would block every launch.
            locked("spark", "server"),
        ],
        unresolved: Vec::new(),
    };

    let missing = missing_client_mods(&lock, &instance);
    std::fs::remove_dir_all(&root).ok();
    assert_eq!(missing, vec!["jade.jar".to_string()]);
}
