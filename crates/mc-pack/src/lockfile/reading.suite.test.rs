use super::super::{LockedLoader, Lockfile};
use super::tests::{lock, locked};
use crate::fixtures::{Workshop, entry};
use mc_mods::Origin;

/// What's written must read back identically: the lockfile is what makes it
/// possible to replay an installation months later, when every version has
/// moved.
#[test]
fn a_written_lockfile_reads_back_identically() {
    let workshop = Workshop::new("lockfile-round-trip");
    let path = workshop.root.join("samflix.lock.json");

    let mut written = crate::fixtures::lock(vec![entry("jei", "both", Some(b"jei"))]);
    written
        .unresolved
        .push(crate::fixtures::missing("ghost", "jei"));
    written.save(&path).expect("write");

    let reread = Lockfile::load(&path).expect("read");
    assert_eq!(reread.name, "samflix");
    assert_eq!(reread.mods.len(), 1);
    assert_eq!(reread.mods[0].file_name, "jei.jar");
    assert_eq!(reread.mods[0].sha1, written.mods[0].sha1);
    assert_eq!(reread.unresolved.len(), 1);
    assert_eq!(reread.unresolved[0].mod_id, "ghost");
}

/// The lockfile is versioned alongside the manifest: a file that ends with a
/// blank line reads better in a review.
#[test]
fn a_written_lockfile_ends_with_a_newline() {
    let workshop = Workshop::new("lockfile-newline");
    let path = workshop.root.join("samflix.lock.json");
    crate::fixtures::lock(Vec::new()).save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.ends_with("}\n"), "{:?}", &text[text.len() - 5..]);
}

/// A missing or broken lockfile must name its path: it's a file you're
/// going to go look at.
#[test]
fn a_missing_or_broken_lockfile_names_its_path() {
    let workshop = Workshop::new("lockfile-broken");

    let missing = workshop.root.join("nowhere.lock.json");
    let error = Lockfile::load(&missing).expect_err("nothing at that spot");
    assert!(format!("{error:#}").contains("nowhere"), "{error:#}");

    let broken = workshop.write("broken.lock.json", b"{this isn't JSON");
    let error = Lockfile::load(&broken).expect_err("invalid JSON");
    assert!(format!("{error:#}").contains("unreadable"), "{error:#}");
}

/// Lockfiles written before `sha512` and `url` existed stay readable:
/// refusing to read them would prevent playing without gaining anything.
#[test]
fn a_lockfile_from_before_the_latest_fields_stays_readable() {
    let raw = r#"{"schema":1,"pack":"samflix","generated":"2025-01-01T00:00:00Z",
      "minecraft":"1.21.1","loader":{"type":"neoforge","version":"21.1.250"},"java":21,
      "mods":[{"slug":"jei","name":"JEI","origin":"modrinth","project":"p","file":"f",
               "version":"1.0","file_name":"jei.jar","sha1":"aa","size":1,"side":"both",
               "reason":"requested","provides":["jei"]}]}"#
        .as_bytes();

    let reread = Lockfile::parse(raw).expect("legacy lockfile");
    assert!(reread.mods[0].url.is_empty());
    assert!(reread.mods[0].sha512.is_none());
    // Their SHA-1 keeps being authoritative until the next `lock`.
    assert_eq!(
        reread.mods[0].checksum(),
        Some(mc_dl::Checksum::Sha1("aa".into()))
    );
    assert!(reread.unresolved.is_empty());
}

/// The lockfile is born from a resolution plan: that's where what was
/// actually installed, and why, gets fixed in place.
#[test]
fn a_resolution_plan_becomes_a_lockfile() {
    use mc_mods::{Plan, Side};

    let plan = Plan {
        mods: Vec::new(),
        unresolved: vec![mc_mods::resolve::Unresolved {
            mod_id: "ghost".into(),
            required_by: "jei".into(),
            side: Side::Both,
        }],
    };

    let manifest = crate::manifest::Manifest::parse(crate::fixtures::MANIFEST.as_bytes())
        .expect("the test manifest reads");

    let lock = Lockfile::from_plan(
        &manifest,
        crate::lockfile::LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        21,
        &plan,
    );

    assert_eq!(lock.name, "samflix");
    assert_eq!(lock.minecraft, "1.21.1");
    assert_eq!(lock.java, 21);
    assert_eq!(lock.unresolved.len(), 1);
    assert_eq!(lock.unresolved[0].mod_id, "ghost");
    // The generation date is set on write, not guessed.
    assert!(lock.generated.ends_with('Z'), "{}", lock.generated);
}

/// A manifest without an extension must not produce a nameless lockfile.
#[test]
fn a_manifest_without_an_extension_still_has_a_lockfile() {
    assert_eq!(
        Lockfile::path_for(std::path::Path::new("packs/samflix")),
        std::path::Path::new("packs/samflix.lock.json")
    );
}

/// Already-published lockfiles use `pack` and `origin`. They must keep
/// reading: the remote file is downloaded on every installation, and
/// refusing to read it would prevent playing until someone republishes it.
#[test]
fn a_lockfile_from_before_the_rename_still_reads() {
    let old = r#"{
        "schema": 1,
        "pack": "samflix",
        "generated": "2026-09-17T00:00:00Z",
        "minecraft": "1.21.1",
        "loader": {"type": "neoforge", "version": "21.1.250"},
        "java": 21,
        "mods": [{
            "slug": "jei",
            "name": "Just Enough Items",
            "origin": "modrinth",
            "project": "u6dRKJwZ",
            "file": "abc",
            "version": "19.51.0",
            "file_name": "jei.jar",
            "size": 1,
            "side": "both",
            "reason": "requested by the manifest",
            "provides": ["jei"]
        }]
    }"#;

    let reread = Lockfile::parse(old.as_bytes()).expect("a lockfile from before stays readable");

    assert_eq!(reread.name, "samflix");
    assert_eq!(reread.mods[0].source, Origin::Modrinth);
    // The channel was missing: we don't assume a pre-release on a pack
    // nobody touched.
    assert_eq!(reread.mods[0].channel, mc_mods::Channel::Release);
    // And the new fields are simply absent.
    assert_eq!(reread.version, None);
    assert!(reread.servers.is_empty());
}

/// What's written from now on uses the manifest's vocabulary, and nothing
/// else: the two files are read one after the other.
#[test]
fn the_written_lockfile_uses_the_manifest_s_vocabulary() {
    let written =
        serde_json::to_string(&lock(vec![locked("jei", "abc", "19.51.0")])).expect("serialization");

    assert!(written.contains("\"name\":"), "{written}");
    assert!(written.contains("\"source\":"), "{written}");
    assert!(!written.contains("\"pack\":"), "{written}");
    assert!(!written.contains("\"origin\":"), "{written}");
}

/// A lockfile alone must be enough: it carries the pack version and the
/// servers, which only the manifest used to give. A server script no longer
/// needs both files.
#[test]
fn the_lockfile_carries_over_what_the_manifest_declares() {
    let manifest = crate::manifest::Manifest::parse(crate::fixtures::MANIFEST.as_bytes())
        .expect("the test manifest reads");

    let lockfile = Lockfile::from_plan(
        &manifest,
        LockedLoader {
            kind: "neoforge".into(),
            version: "21.1.250".into(),
        },
        21,
        &mc_mods::Plan {
            mods: Vec::new(),
            unresolved: Vec::new(),
        },
    );

    assert_eq!(lockfile.name, manifest.name);
    assert_eq!(lockfile.version, manifest.version);
    assert_eq!(
        lockfile.servers.keys().collect::<Vec<_>>(),
        manifest.servers.keys().collect::<Vec<_>>()
    );
    assert_eq!(lockfile.minecraft, manifest.minecraft);
}

/// Lockfiles published before `generation` existed read as generation 0.
/// Refusing to read them would prevent playing until someone republishes
/// them — exactly the failure a forced-reinstall mechanism must not cause.
#[test]
fn a_lockfile_from_before_generation_reads_as_zero() {
    let old = r#"{"schema":1,"name":"samflix","generated":"2026-09-17T00:00:00Z",
      "minecraft":"1.21.1","loader":{"type":"neoforge","version":"21.1.250"},"java":21,
      "mods":[]}"#;

    let reread = Lockfile::parse(old.as_bytes()).expect("a lockfile from before stays readable");
    assert_eq!(reread.generation, 0);
}

/// And the generation is ALWAYS written, even at zero: no
/// `skip_serializing_if`. Without that, it would be necessary to
/// distinguish “never placed” from “placed at zero”, when both mean the
/// same thing.
#[test]
fn generation_is_written_even_at_zero() {
    let written = serde_json::to_string(&lock(Vec::new())).expect("serialization");
    assert!(written.contains("\"generation\":0"), "{written}");
}

/// The digest is computed on what the lockfile SAYS, not on how it's
/// written.
///
/// It's the property that prevents redownloading eight hundred megabytes
/// because the host changed its indentation: the same lockfile minified and
/// the same lockfile indented give the same digest.
#[test]
fn the_digest_ignores_formatting() {
    let indented = r#"{
        "schema": 1,
        "name": "samflix",
        "generated": "2026-09-17T00:00:00Z",
        "minecraft": "1.21.1",
        "loader": { "type": "neoforge", "version": "21.1.250" },
        "java": 21,
        "mods": []
    }"#;
    let minified = r#"{"schema":1,"name":"samflix","generated":"2026-09-17T00:00:00Z","minecraft":"1.21.1","loader":{"type":"neoforge","version":"21.1.250"},"java":21,"mods":[]}"#;

    let a = Lockfile::parse(indented.as_bytes())
        .unwrap()
        .digest()
        .unwrap();
    let b = Lockfile::parse(minified.as_bytes())
        .unwrap()
        .digest()
        .unwrap();
    assert_eq!(a, b);
}

/// But it doesn't miss a real change: a mod version that moves must give a
/// different digest, or the launcher would never see an update.
#[test]
fn the_digest_sees_a_real_change() {
    let before = lock(vec![locked("jei", "abc", "19.51.0")]);
    let after = lock(vec![locked("jei", "def", "19.52.0")]);
    assert_ne!(before.digest().unwrap(), after.digest().unwrap());
}

/// And a generation change, with identical mods, changes the digest too:
/// it's what makes a pack due for a purge distinguishable from an unchanged
/// pack.
#[test]
fn the_digest_sees_a_generation_change() {
    let before = lock(Vec::new());
    let mut after = lock(Vec::new());
    after.generation = 1;
    assert_ne!(before.digest().unwrap(), after.digest().unwrap());
}

/// The digest of a lockfile written then reread is that of the original
/// lockfile. It's the condition for `save` and `digest` to stay
/// consistent: they go through the same serialization, not two that merely
/// look alike.
#[test]
fn writing_then_rereading_preserves_the_digest() {
    let workshop = Workshop::new("lockfile-digest");
    let path = workshop.root.join("samflix.lock.json");

    let written = crate::fixtures::lock(vec![entry("jei", "both", Some(b"jei"))]);
    written.save(&path).expect("write");

    let reread = Lockfile::load(&path).expect("read");
    assert_eq!(written.digest().unwrap(), reread.digest().unwrap());
}
