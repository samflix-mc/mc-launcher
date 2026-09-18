use super::verify;
use crate::Options;
use crate::fixtures::{MANIFEST, Workshop, entry, lock, missing};
use crate::lockfile::{LockedMod, Lockfile};
use crate::source::Source;

const JAR: &[u8] = b"the jar";

/// A complete installation: the pack, its lockfile, and the Mojang files
/// `mc_instance::verify` requires.
fn installation(name: &str, mods: Vec<LockedMod>) -> (Workshop, Options, Source) {
    let workshop = Workshop::new(name);
    let manifest_path = workshop.write("samflix.json", MANIFEST.as_bytes());
    lock(mods)
        .save(&workshop.root.join("samflix.lock.json"))
        .unwrap();

    let options = Options {
        layout: mc_instance::Layout::new(workshop.root.join("data")),
        instance_name: Some("samflix".into()),
        ..Default::default()
    };

    // What mc-instance verifies on its own side: descriptors and client.
    let shared = options.layout.shared();
    for (path, content) in [
        ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
        ("versions/1.21.1/1.21.1.jar", "jar"),
        (
            "versions/neoforge-21.1.250/neoforge-21.1.250.json",
            r#"{"libraries":[]}"#,
        ),
    ] {
        let target = shared.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, content).unwrap();
    }

    let source = Source::parse(manifest_path.to_str().unwrap(), &options.layout);
    (workshop, options, source)
}

/// Places a jar in the `mods` folder on the requested side.
fn place(options: &Options, side: &str, name: &str, content: &[u8]) {
    let instance = options.layout.instance("samflix");
    let folder = match side {
        "client" => instance.mods_dir(),
        _ => instance.dir.join("server").join("mods"),
    };
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join(name), content).unwrap();
}

#[test]
fn a_complete_installation_reports_nothing() {
    let (_workshop, options, source) =
        installation("verify-ok", vec![entry("jei", "both", Some(JAR))]);
    place(&options, "client", "jei.jar", JAR);
    place(&options, "server", "jei.jar", JAR);

    let problems = verify(&source, &options, false).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
}

/// A client mod has no business in the server's folder, and vice versa:
/// checking it on both sides would raise a false alarm on a correct
/// installation.
#[test]
fn a_client_mod_is_only_looked_for_on_the_client_side() {
    let (_workshop, options, source) =
        installation("verify-side", vec![entry("sodium", "client", Some(JAR))]);
    place(&options, "client", "sodium.jar", JAR);

    let problems = verify(&source, &options, false).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_missing_mod_is_named_by_its_path() {
    let (_workshop, options, source) =
        installation("verify-missing", vec![entry("jei", "both", Some(JAR))]);
    place(&options, "client", "jei.jar", JAR);
    // Nothing on the server side.

    let problems = verify(&source, &options, false).unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("missing mod"), "{problems:?}");
    assert!(problems[0].contains("jei.jar"), "{problems:?}");
}

/// A tampered jar must be caught: that's the whole point of keeping the
/// digest in the lockfile.
#[test]
fn an_altered_mod_is_reported_with_both_digests() {
    let (_workshop, options, source) =
        installation("verify-altered", vec![entry("jei", "client", Some(JAR))]);
    place(&options, "client", "jei.jar", b"something else");

    let problems = verify(&source, &options, false).unwrap();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("digest"), "{problems:?}");
}

/// A lockfile with no digest can't be verified: that's the case for entries
/// written from a source that didn't publish one.
#[test]
fn a_mod_without_a_digest_is_only_confirmed_present() {
    let (_workshop, options, source) =
        installation("verify-no-digest", vec![entry("jei", "client", None)]);
    place(&options, "client", "jei.jar", b"doesn't matter");

    let problems = verify(&source, &options, false).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
}

/// The lockfile carries the `modId`s each jar provides: the whole's coherence
/// is checked without reopening a single archive.
#[test]
fn an_unmet_dependency_is_reported_without_opening_a_jar() {
    let workshop = Workshop::new("verify-coherence");
    let manifest_path = workshop.write("samflix.json", MANIFEST.as_bytes());
    let mut lockfile = lock(vec![entry("jei", "client", Some(JAR))]);
    lockfile.unresolved.push(missing("bookshelf", "jei"));
    lockfile
        .save(&workshop.root.join("samflix.lock.json"))
        .unwrap();

    let options = Options {
        layout: mc_instance::Layout::new(workshop.root.join("data")),
        instance_name: Some("samflix".into()),
        ..Default::default()
    };
    let shared = options.layout.shared();
    for (path, content) in [
        ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
        ("versions/1.21.1/1.21.1.jar", "jar"),
        (
            "versions/neoforge-21.1.250/neoforge-21.1.250.json",
            r#"{"libraries":[]}"#,
        ),
    ] {
        let target = shared.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, content).unwrap();
    }
    place(&options, "client", "jei.jar", JAR);

    let source = Source::parse(manifest_path.to_str().unwrap(), &options.layout);
    let problems = verify(&source, &options, false).unwrap();

    assert!(
        problems
            .iter()
            .any(|p| p.contains("unmet dependency") && p.contains("bookshelf")),
        "{problems:?}"
    );
}

/// A gap the lockfile records but another jar ends up providing isn't one
/// anymore: reporting it would send someone chasing a problem that's already
/// solved.
#[test]
fn a_gap_eventually_filled_is_no_longer_a_problem() {
    let workshop = Workshop::new("verify-gap-filled");
    let manifest_path = workshop.write("samflix.json", MANIFEST.as_bytes());
    let mut lockfile = lock(vec![entry("bookshelf", "client", Some(JAR))]);
    lockfile.unresolved.push(missing("bookshelf", "jei"));
    lockfile
        .save(&workshop.root.join("samflix.lock.json"))
        .unwrap();

    let options = Options {
        layout: mc_instance::Layout::new(workshop.root.join("data")),
        instance_name: Some("samflix".into()),
        ..Default::default()
    };
    let shared = options.layout.shared();
    for (path, content) in [
        ("versions/1.21.1/1.21.1.json", r#"{"libraries":[]}"#),
        ("versions/1.21.1/1.21.1.jar", "jar"),
        (
            "versions/neoforge-21.1.250/neoforge-21.1.250.json",
            r#"{"libraries":[]}"#,
        ),
    ] {
        let target = shared.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, content).unwrap();
    }
    place(&options, "client", "bookshelf.jar", JAR);

    let source = Source::parse(manifest_path.to_str().unwrap(), &options.layout);
    let problems = verify(&source, &options, false).unwrap();

    assert!(
        !problems.iter().any(|p| p.contains("unmet")),
        "{problems:?}"
    );
}

/// With no lockfile, there's nothing to verify — and the message must say
/// what to do.
#[test]
fn without_a_lockfile_verification_points_to_the_install() {
    let workshop = Workshop::new("verify-no-lock");
    let manifest_path = workshop.write("samflix.json", MANIFEST.as_bytes());
    let options = Options {
        layout: mc_instance::Layout::new(workshop.root.join("data")),
        ..Default::default()
    };
    let source = Source::parse(manifest_path.to_str().unwrap(), &options.layout);

    let error = verify(&source, &options, false).expect_err("no lockfile");
    assert!(
        format!("{error:#}").contains("mc-pack install"),
        "{error:#}"
    );
}

/// Absent a name, the instance takes the pack's — otherwise verification
/// would look at an empty directory and raise an alarm on everything.
#[test]
fn absent_a_name_the_instance_takes_the_packs() {
    let (_workshop, mut options, source) =
        installation("verify-name", vec![entry("jei", "client", Some(JAR))]);
    options.instance_name = None;
    place(&options, "client", "jei.jar", JAR);

    let problems = verify(&source, &options, false).unwrap();
    assert!(problems.is_empty(), "{problems:?}");
}

/// The lockfile read back must be the one we wrote: a pack whose lockfile was
/// replaced by something else must not pass as verified.
#[test]
fn the_lockfile_read_back_is_indeed_the_packs() {
    let (workshop, _options, _source) =
        installation("verify-identity", vec![entry("jei", "client", Some(JAR))]);
    let reread = Lockfile::load(&workshop.root.join("samflix.lock.json")).unwrap();
    assert_eq!(reread.name, "samflix");
}
