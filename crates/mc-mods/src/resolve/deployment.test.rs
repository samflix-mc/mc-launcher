use super::deploy;
use crate::fixtures::Workshop;
use crate::jar::Side;
use crate::resolve::fixtures::{candidate, installed};
use crate::resolve::plan::{Installed, Plan};

/// A mod whose jar exists in the cache, ready to be placed.
fn place(workshop: &Workshop, slug: &str, content: &[u8], side: Side) -> Installed {
    let cache = workshop.root.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let path = cache.join(format!("{slug}.jar"));
    std::fs::write(&path, content).unwrap();

    let mut entry = installed(slug, &[], &[]);
    entry.candidate = candidate(slug, "1.0");
    entry.candidate.sha1 = Some(mc_dl::Checksum::Sha1(String::new()).of(content));
    entry.path = path;
    entry.side = side;
    entry
}

fn plan(mods: Vec<Installed>) -> Plan {
    Plan {
        mods,
        unresolved: Vec::new(),
    }
}

#[test]
fn the_plans_jars_land_in_the_mods_folder() {
    let workshop = Workshop::new("deploy-simple");
    let mods_dir = workshop.root.join("mods");
    let plan = plan(vec![
        place(&workshop, "jei", b"jei", Side::Both),
        place(&workshop, "sodium", b"sodium", Side::Client),
    ]);

    let deployed = deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(deployed.installed, 2);
    assert!(deployed.removed.is_empty());
    assert!(mods_dir.join("jei.jar").is_file());
    assert!(mods_dir.join("sodium.jar").is_file());
}

/// A client mod in the server's folder would stay loaded and would drift the
/// server's registry from the client's.
#[test]
fn a_client_mod_does_not_go_to_the_server() {
    let workshop = Workshop::new("deploy-side");
    let mods_dir = workshop.root.join("mods");
    let plan = plan(vec![
        place(&workshop, "jei", b"jei", Side::Both),
        place(&workshop, "sodium", b"sodium", Side::Client),
    ]);

    let deployed = deploy(&plan, Side::Server, &mods_dir).unwrap();

    assert_eq!(deployed.installed, 1);
    assert!(mods_dir.join("jei.jar").is_file());
    assert!(!mods_dir.join("sodium.jar").exists());
}

/// What the plan no longer contains must disappear: a mod removed from the
/// manifest but left on disk would stay loaded by the game.
#[test]
fn a_jar_no_longer_in_the_plan_is_removed() {
    let workshop = Workshop::new("deploy-removal");
    let mods_dir = workshop.root.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();
    std::fs::write(mods_dir.join("old.jar"), b"old").unwrap();
    // Whatever isn't a jar is none of our concern: configuration, notes.
    std::fs::write(mods_dir.join("notes.txt"), b"keep this").unwrap();

    let plan = plan(vec![place(&workshop, "jei", b"jei", Side::Both)]);
    let deployed = deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(deployed.removed, vec!["old.jar"]);
    assert!(!mods_dir.join("old.jar").exists());
    assert!(mods_dir.join("notes.txt").is_file());
}

/// A jar already placed and conformant isn't recopied: a pack weighs several
/// hundred megabytes, and the next launch would be needlessly long.
#[test]
fn an_already_conformant_jar_is_left_in_place() {
    let workshop = Workshop::new("deploy-idempotent");
    let mods_dir = workshop.root.join("mods");
    let plan = plan(vec![place(&workshop, "jei", b"jei", Side::Both)]);

    deploy(&plan, Side::Client, &mods_dir).unwrap();
    let inode = std::fs::metadata(mods_dir.join("jei.jar")).unwrap();
    let deployed = deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(deployed.installed, 1);
    assert_eq!(
        std::fs::metadata(mods_dir.join("jei.jar")).unwrap().len(),
        inode.len()
    );
}

/// A jar present but altered — edited by hand, interrupted download — must
/// be replaced by the one from the cache.
#[test]
fn an_altered_jar_is_replaced() {
    let workshop = Workshop::new("deploy-altered");
    let mods_dir = workshop.root.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();
    std::fs::write(mods_dir.join("jei.jar"), b"something else").unwrap();

    let plan = plan(vec![place(&workshop, "jei", b"jei", Side::Both)]);
    deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(std::fs::read(mods_dir.join("jei.jar")).unwrap(), b"jei");
}

/// Without a published digest, a file already there is accepted as is:
/// replacing it on every launch would prove nothing more.
#[test]
fn without_a_digest_a_jar_already_there_is_accepted() {
    let workshop = Workshop::new("deploy-no-digest");
    let mods_dir = workshop.root.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();
    std::fs::write(mods_dir.join("jei.jar"), b"placed by hand").unwrap();

    let mut entry = place(&workshop, "jei", b"jei", Side::Both);
    entry.candidate.sha1 = None;
    entry.candidate.sha512 = None;

    deploy(&plan(vec![entry]), Side::Client, &mods_dir).unwrap();

    assert_eq!(
        std::fs::read(mods_dir.join("jei.jar")).unwrap(),
        b"placed by hand"
    );
}

#[test]
fn the_mods_folder_is_created_as_needed() {
    let workshop = Workshop::new("deploy-creation");
    let mods_dir = workshop
        .root
        .join("instance")
        .join("minecraft")
        .join("mods");

    let deployed = deploy(&plan(Vec::new()), Side::Client, &mods_dir).unwrap();

    assert!(mods_dir.is_dir());
    assert_eq!(deployed.installed, 0);
}
