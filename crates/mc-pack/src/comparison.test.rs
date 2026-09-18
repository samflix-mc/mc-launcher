use super::{Action, Drift, compare, presence, published_lock};
use crate::fixtures::Workshop;
use crate::source::Source;
use crate::state::LocalState;

/// A published lock, served as-is.
fn lock_body(generation: u32) -> String {
    format!(
        r#"{{"schema":1,"name":"samflix","version":"3.2","generated":"2026-09-18T00:00:00Z",
           "minecraft":"1.21.1","loader":{{"type":"neoforge","version":"21.1.250"}},
           "java":21,"generation":{generation},"mods":[]}}"#
    )
}

/// Places an instance that looks installed, with the state given to it.
fn place_instance(workshop: &Workshop, digest: &str, generation: u32) {
    let options = workshop.options();
    let instance = options.layout.instance("samflix");
    std::fs::create_dir_all(instance.mods_dir()).unwrap();
    LocalState::new(digest.into(), generation, "2026-09-18T00:00:00Z".into())
        .write(&crate::state::path(&instance))
        .unwrap();
}

// --- presence ----------------------------------------------------------

/// On a clean machine, nothing is placed. This is the repo's third piece of
/// debt: `State.installation` used to be populated only by an installation
/// done in the current session, so on startup a perfectly installed pack
/// looked absent.
#[test]
fn on_a_clean_machine_nothing_is_placed() {
    let workshop = Workshop::new("presence-clean");
    let seen = presence(&workshop.options(), "samflix");
    assert!(!seen.installed);
    assert_eq!(seen.state, None);
}

/// A complete instance is recognized WITHOUT an installation having happened
/// in this session. That's the whole point of the function.
#[test]
fn a_complete_instance_is_recognized_on_startup() {
    let workshop = Workshop::new("presence-complete");
    place_instance(&workshop, "aa", 0);

    let seen = presence(&workshop.options(), "samflix");
    assert!(seen.installed);
    assert_eq!(seen.state.unwrap().lock_sha512, "aa");
}

/// A state without a mods directory describes an installation someone
/// half-erased by hand. Saying "installed" would offer PLAY on an empty
/// directory, and the game would crash on launch.
#[test]
fn a_state_without_mods_does_not_count_as_installed() {
    let workshop = Workshop::new("presence-half");
    let options = workshop.options();
    let instance = options.layout.instance("samflix");
    std::fs::create_dir_all(&instance.dir).unwrap();
    LocalState::new("aa".into(), 0, "2026-09-18T00:00:00Z".into())
        .write(&crate::state::path(&instance))
        .unwrap();

    assert!(!presence(&options, "samflix").installed);
}

/// Nor do mods without a state: that's an installation from before this
/// version, about which nothing is known. It'll go through a full
/// installation once — which is a hundred percent of the population.
#[test]
fn mods_without_state_do_not_count_as_installed() {
    let workshop = Workshop::new("presence-legacy");
    let options = workshop.options();
    std::fs::create_dir_all(options.layout.instance("samflix").mods_dir()).unwrap();

    assert!(!presence(&options, "samflix").installed);
}

// --- published_lock -----------------------------------------------------

/// The published lock is fetched, and it's really it that's read.
#[tokio::test]
async fn the_published_lock_is_fetched() {
    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &lock_body(0));
    let dl = mc_dl::Downloader::new("test").unwrap();

    let lock = published_lock(&format!("{}/pack/samflix.json", server.base()), &dl)
        .await
        .expect("the published lock reads");

    assert_eq!(lock.name, "samflix");
    assert_eq!(lock.java, 21);
}

/// THE boundary this module exists to hold: a comparison writes NOTHING to
/// the cache.
///
/// The only existing path, `load_remote`, saves the manifest and the lock
/// before the slightest installation has begun. Reusing it would make the
/// cache the description of the PUBLISHED pack instead of the PLACED one,
/// and would break offline verification, which compares the disk to that
/// cache.
///
/// This test is the only way to hold the boundary over time: it doesn't
/// show in a signature.
#[tokio::test]
async fn a_comparison_writes_nothing_to_the_cache() {
    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &lock_body(0));
    let workshop = Workshop::new("compare-no-write");
    let options = workshop.options();

    let cache = options.layout.cache();
    std::fs::create_dir_all(&cache).unwrap();
    let before = recursive_contents(&cache);

    let source = Source::Remote {
        url: format!("{}/pack/samflix.json", server.base()),
        cache_dir: cache.clone(),
    };
    let dl = mc_dl::Downloader::new("test").unwrap();
    compare(&source, &options, &dl).await;

    assert_eq!(
        recursive_contents(&cache),
        before,
        "the comparison wrote to the cache"
    );
}

fn recursive_contents(root: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(recursive_contents(&path));
        } else {
            found.push(path.display().to_string());
        }
    }
    found.sort();
    found
}

// --- compare -------------------------------------------------------------

/// Nothing placed: the button says INSTALL.
#[tokio::test]
async fn with_nothing_placed_the_button_says_install() {
    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &lock_body(0));
    let workshop = Workshop::new("compare-absent");
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", server.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.action, Action::Install);
    assert_eq!(seen.drift, Drift::Absent);
    assert!(!seen.installed);
    assert!(!seen.offline);
    // The lock feeds the Settings page even when nothing is placed.
    assert_eq!(seen.java, Some(21));
    assert_eq!(seen.version.as_deref(), Some("3.2"));
}

/// Placed and identical to the published pack: up to date, and playing will
/// download nothing.
#[tokio::test]
async fn a_pack_identical_to_the_published_one_is_up_to_date() {
    let body = lock_body(0);
    let digest = crate::lockfile::Lockfile::parse(body.as_bytes())
        .unwrap()
        .digest()
        .unwrap();

    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &body);
    let workshop = Workshop::new("compare-up-to-date");
    place_instance(&workshop, &digest, 0);
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", server.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.action, Action::Play);
    assert_eq!(seen.drift, Drift::UpToDate);
}

/// Placed but different: an update by diff.
///
/// **And the button says INSTALL, not PLAY.** It used to say PLAY while the
/// gesture was single — "Update and play" placed the update then launched
/// Minecraft. Sam pushed back on this during acceptance testing: as soon as
/// there's something to place, the gesture is to place, and playing needs a
/// second click.
#[tokio::test]
async fn a_lock_that_moved_asks_for_an_update() {
    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &lock_body(0));
    let workshop = Workshop::new("compare-update");
    place_instance(&workshop, "a-previous-digest", 0);
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", server.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.action, Action::Install);
    assert_eq!(seen.drift, Drift::Update);
}

/// Generation wins over digest: even if the locks matched, a higher
/// generation is an explicit request for reinstallation.
#[tokio::test]
async fn a_higher_generation_wins_over_the_digest() {
    let body = lock_body(2);
    let digest = crate::lockfile::Lockfile::parse(body.as_bytes())
        .unwrap()
        .digest()
        .unwrap();

    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &body);
    let workshop = Workshop::new("compare-generation");
    // Same digest, lower generation.
    place_instance(&workshop, &digest, 1);
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", server.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.drift, Drift::Reinstall);
    assert_eq!(seen.generation, 2);
}

/// Without network, this is NOT an error: the button says PLAY if something
/// is placed. Refusing to play because verification failed would punish a
/// player for a network outage.
#[tokio::test]
async fn without_network_a_placed_pack_stays_playable() {
    let workshop = Workshop::new("compare-offline");
    place_instance(&workshop, "aa", 0);
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::Remote {
            // An address that won't respond.
            url: "http://127.0.0.1:1/pack/samflix.json".into(),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.action, Action::Play);
    assert_eq!(seen.drift, Drift::Unknown);
    assert!(seen.offline);
    assert!(seen.installed);
}

/// Without network and with nothing placed, the button says INSTALL — and
/// the installation will fail outright for lack of a pack, which is the
/// right way to fail: the player learns they need a network.
#[tokio::test]
async fn without_network_and_with_nothing_the_button_says_install() {
    let workshop = Workshop::new("compare-offline-clean");
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::Remote {
            url: "http://127.0.0.1:1/pack/samflix.json".into(),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.action, Action::Install);
    assert!(seen.offline);
}

/// A LOCAL pack has no "published" counterpart: it's the file being edited
/// that's authoritative. We don't claim to know whether there's an update,
/// and the button goes by presence alone.
#[tokio::test]
async fn a_local_pack_does_not_claim_to_know_a_published_one() {
    let workshop = Workshop::new("compare-local");
    place_instance(&workshop, "aa", 0);
    let options = workshop.options();
    let dl = mc_dl::Downloader::new("test").unwrap();

    let seen = compare(
        &Source::File {
            manifest: workshop.root.join("samflix.json"),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(seen.drift, Drift::Unknown);
    assert_eq!(seen.action, Action::Play);
}

/// **The button's rule, case by case.**
///
/// It fits in three lines and decides what happens on a click: placing eight
/// hundred megabytes, or launching Minecraft. A table rather than separate
/// tests because it's the COVERAGE of the cases that matters here, and a
/// table can be scanned at a glance to check none is missing.
#[test]
fn the_button_places_as_soon_as_there_is_something_to_place() {
    // (drift, offline, installed) → expected action
    let cases = [
        // Nothing placed: install, whatever the rest.
        (Drift::Absent, false, false, Action::Install),
        (Drift::Unknown, true, false, Action::Install),
        // Placed and up to date: play. The common case, and the only one.
        (Drift::UpToDate, false, true, Action::Play),
        // Placed but stale: PLACE. The button no longer launches the game
        // right after — that's Sam's feedback from acceptance testing.
        (Drift::Update, false, true, Action::Install),
        (Drift::Reinstall, false, true, Action::Install),
        // Offline with a pack placed: play. We couldn't verify, and
        // punishing someone whose pack is there for a network outage would
        // be absurd.
        (Drift::Unknown, true, true, Action::Play),
    ];

    for (drift, offline, installed, expected) in cases {
        assert_eq!(
            super::action_for(drift, offline, installed),
            expected,
            "drift={drift:?}, offline={offline}, installed={installed}"
        );
    }
}

/// `should_place` is the SAME rule the installation follows.
///
/// The two lived separately — one in `comparison`, the other in
/// `game::sequence` — and that's exactly the kind of duplicate that drifts
/// apart silently: the button would offer to play a pack the installation
/// just decided to re-place.
#[test]
fn the_buttons_rule_is_the_catch_up_rule() {
    for (drift, offline) in [
        (Drift::Absent, false),
        (Drift::UpToDate, false),
        (Drift::Update, false),
        (Drift::Reinstall, false),
        (Drift::Unknown, false),
        (Drift::Update, true),
    ] {
        let state = super::PackState {
            action: Action::Play,
            drift,
            offline,
            installed: true,
            name: None,
            version: None,
            java: None,
            mods: 0,
            generation: 0,
        };
        assert_eq!(
            super::should_place(drift, offline),
            crate::game::should_catch_up(&state),
            "drift={drift:?}, offline={offline}"
        );
    }
}

/// **The witness survives the instance's disappearance, and it was lying.**
///
/// Real scenario, reported by Sam: an `rm -rf` on
/// `instances/samflix/minecraft/` to start clean. `state.json` lives one
/// level up — it stayed behind.
///
/// The drift used to be computed from that witness alone: the placed digest
/// equal to the published one, hence "up to date". The button said INSTALL
/// anyway, because its `Action` looks at PRESENCE. The two disagreed, and
/// it's the drift that drives the installation: clicking reported "the pack
/// was already up to date: nothing to place", on an empty directory. Before
/// the gestures were split, the game would launch on that.
#[tokio::test]
async fn an_instance_erased_by_hand_gets_reinstalled() {
    let body = lock_body(0);
    let digest = crate::lockfile::Lockfile::parse(body.as_bytes())
        .unwrap()
        .digest()
        .unwrap();

    let server = mc_testkit::Server::new().await;
    server.json("/pack/samflix.lock.json", &body);
    let workshop = Workshop::new("compare-erased-instance");
    place_instance(&workshop, &digest, 0);

    // What the hand of someone wanting to "start from scratch" does: the
    // game directory goes away, the witness stays.
    let options = workshop.options();
    let instance = options.layout.instance("samflix");
    std::fs::remove_dir_all(instance.mods_dir()).unwrap();
    assert!(
        crate::state::path(&instance).exists(),
        "the witness must survive, otherwise this test proves nothing"
    );

    let dl = mc_dl::Downloader::new("test").unwrap();
    let seen = compare(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", server.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert!(!seen.installed);
    assert_eq!(seen.drift, Drift::Absent);
    assert_eq!(seen.action, Action::Install);
    // And above all: the installation, which reads the drift, has something
    // to do.
    assert!(super::should_place(seen.drift, seen.offline));
}
