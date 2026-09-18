use super::{resolve, resolve_with};
use crate::fixtures::{
    Project, Version, Workshop, jar, jar_with_bundled, jar_with_two_bundled, publish,
};
use crate::jar::Side;
use crate::resolve::registry::Registry;
use crate::resolve::request::Request;
use crate::resolve::{Options, Plan};

const MC: &str = "1.21.1";
const LOADER: &str = "neoforge";

/// A registry whose Modrinth is the test server, and a fresh cache.
fn registry(workshop: &Workshop, server: &mc_testkit::Server) -> Registry {
    Registry::for_fixtures(workshop.root.join("cache"), &server.base())
        .expect("the registry builds")
}

fn requests(slugs: &[&str]) -> Vec<Request> {
    slugs.iter().map(|s| Request::new(*s)).collect()
}

fn retained(plan: &Plan) -> Vec<&str> {
    plan.mods
        .iter()
        .map(|m| m.candidate.slug.as_str())
        .collect()
}

/// The nominal case, and what it must produce: the jar on disk, its digest
/// recomputed since the manifest didn't give one, and what the jar's
/// descriptor declares it provides.
#[tokio::test]
async fn a_mod_without_a_dependency_is_resolved_downloaded_and_read() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("simple");

    let content = jar("jei", &[]);
    server.bytes("/jei.jar", &content);
    publish(
        &server,
        &Project::new("jei").version(Version::new("19.51.0", &server.url("/jei.jar"), &content)),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["jei"]),
        MC,
        LOADER,
    )
    .await
    .expect("resolution succeeds");

    assert_eq!(retained(&plan), vec!["jei"]);
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);

    let kept = &plan.mods[0];
    assert!(kept.path.is_file(), "{}", kept.path.display());
    assert!(kept.provides.contains("jei"));
    // The cache is indexed by source and by project: two different mods
    // sometimes publish a jar under the same name.
    assert!(kept.path.to_string_lossy().contains("modrinth"));
}

/// Dependencies the author enters at publish time: the ones the API
/// announces, which the resolver follows without having to open the jar.
#[tokio::test]
async fn a_dependency_declared_by_the_api_is_installed() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("declared");

    let library = jar("bookshelf", &[]);
    server.bytes("/bookshelf.jar", &library);
    publish(
        &server,
        &Project::new("bookshelf").version(Version::new(
            "20.2.0",
            &server.url("/bookshelf.jar"),
            &library,
        )),
    );

    let main = jar("enchantments", &[]);
    server.bytes("/enchantments.jar", &main);
    publish(
        &server,
        &Project::new("enchantments").version(
            Version::new("1.0", &server.url("/enchantments.jar"), &main).declare("bookshelf"),
        ),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["enchantments"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retained(&plan), vec!["bookshelf", "enchantments"]);
}

/// The case that justifies the whole crate: an author adds a library
/// between two versions and doesn't go back to edit the publish page. The
/// API declares nothing; the jar, on the other hand, requires it — and
/// without it the game stops on "Missing or unsupported mods".
#[tokio::test]
async fn a_dependency_only_the_jar_declares_is_caught_up() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("implicit");

    let library = jar("bookshelf", &[]);
    server.bytes("/bookshelf.jar", &library);
    publish(
        &server,
        &Project::new("bookshelf").version(Version::new(
            "20.2.0",
            &server.url("/bookshelf.jar"),
            &library,
        )),
    );

    // The jar requires "bookshelf"; the published version doesn't declare it.
    let main = jar("enchantments", &[("bookshelf", "BOTH")]);
    server.bytes("/enchantments.jar", &main);
    publish(
        &server,
        &Project::new("enchantments").version(Version::new(
            "1.0",
            &server.url("/enchantments.jar"),
            &main,
        )),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["enchantments"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retained(&plan), vec!["bookshelf", "enchantments"]);
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);
}

/// A mod can bundle its libraries via JarJar. Ignoring them would conclude a
/// dependency is missing and install a duplicate — two versions of the same
/// mod, which NeoForge refuses.
#[tokio::test]
async fn a_bundled_library_does_not_cause_a_duplicate() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("jarjar");

    let content = jar_with_bundled("architectury", "cloth-config");
    server.bytes("/architectury.jar", &content);
    publish(
        &server,
        &Project::new("architectury").version(Version::new(
            "13.0.0",
            &server.url("/architectury.jar"),
            &content,
        )),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["architectury"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retained(&plan), vec!["architectury"]);
    // The bundled jar is counted as a contribution, not as an identity:
    // this is what lets another mod bundle the same library without
    // looking like a duplicate.
    assert!(
        plan.mods[0].bundled.contains("cloth-config"),
        "the bundled jar isn't counted: {:?}",
        plan.mods[0].bundled
    );
    assert!(
        !plan.mods[0].provides.contains("cloth-config"),
        "a bundled modId must not become the mod's identity"
    );
    assert!(plan.mods[0].supplies().any(|id| id == "cloth-config"));
}

/// What nobody can supply doesn't stop the install: it's the caller who
/// decides, and it needs to know which mod claimed what.
#[tokio::test]
async fn a_missing_modid_is_recorded_without_stopping_everything() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("not-found");

    let content = jar("enchantments", &[("ghost-mod", "BOTH")]);
    server.bytes("/enchantments.jar", &content);
    publish(
        &server,
        &Project::new("enchantments").version(Version::new(
            "1.0",
            &server.url("/enchantments.jar"),
            &content,
        )),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["enchantments"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(retained(&plan), vec!["enchantments"]);
    assert_eq!(plan.unresolved.len(), 1, "{:?}", plan.unresolved);
    assert_eq!(plan.unresolved[0].mod_id, "ghost-mod");
    assert_eq!(plan.unresolved[0].required_by, "enchantments");
}

/// Turning off tracking of declared dependencies breaks nothing: catch-up
/// by reading the jars finds the same ones, one pass later. This is what
/// lets us rely solely on what the game will read, when a publish page is
/// at fault.
#[tokio::test]
async fn ignoring_declared_dependencies_loses_nothing() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("without-declared");

    let library = jar("bookshelf", &[]);
    server.bytes("/bookshelf.jar", &library);
    publish(
        &server,
        &Project::new("bookshelf").version(Version::new(
            "20.2.0",
            &server.url("/bookshelf.jar"),
            &library,
        )),
    );

    let main = jar("enchantments", &[("bookshelf", "BOTH")]);
    server.bytes("/enchantments.jar", &main);
    publish(
        &server,
        &Project::new("enchantments").version(
            Version::new("1.0", &server.url("/enchantments.jar"), &main).declare("bookshelf"),
        ),
    );

    let plan = resolve_with(
        &registry(&workshop, &server),
        &requests(&["enchantments"]),
        MC,
        LOADER,
        Options {
            follow_declared: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(retained(&plan), vec!["bookshelf", "enchantments"]);
}

/// Modrinth publishes `client_side` / `server_side` per project: this is
/// what gives the split without having to enter it in the manifest.
#[tokio::test]
async fn the_side_published_by_the_project_is_kept() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("side");

    let content = jar("sodium", &[]);
    server.bytes("/sodium.jar", &content);
    publish(
        &server,
        &Project::new("sodium")
            .sides("required", "unsupported")
            .version(Version::new("0.6", &server.url("/sodium.jar"), &content)),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["sodium"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(plan.mods[0].side, Side::Client);
    assert_eq!(plan.for_side(Side::Client).count(), 1);
    assert_eq!(plan.for_side(Side::Server).count(), 0);
}

/// A mod not found in any source stops resolution: the manifest requests it
/// explicitly, ignoring it would give an incomplete pack with nothing to
/// say so.
#[tokio::test]
async fn a_missing_manifest_mod_stops_everything() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("absent");

    let error = resolve(
        &registry(&workshop, &server),
        &requests(&["mod-that-does-not-exist"]),
        MC,
        LOADER,
    )
    .await
    .expect_err("no source knows it");

    let text = format!("{error:#}");
    assert!(text.contains("mod-that-does-not-exist"), "{text}");
    // CurseForge's keyword search is closed: a slug that doesn't match the
    // one on the site can't be found there, and that's the most frequent
    // cause. The message therefore sends the reader to check the slug.
    assert!(text.contains("slug"), "{text}");
}

/// The default channel is `release`; a beta must not invite itself into a
/// pack that doesn't ask for one.
#[tokio::test]
async fn a_beta_is_not_kept_unless_requested() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("channel");

    let stable = jar("jei", &[]);
    let beta = jar("jei", &[]);
    server.bytes("/jei-stable.jar", &stable);
    server.bytes("/jei-beta.jar", &beta);
    publish(
        &server,
        &Project::new("jei")
            .version(
                Version::new("19.0.0", &server.url("/jei-stable.jar"), &stable)
                    .published("2026-01-01T00:00:00Z"),
            )
            .version(
                Version::new("20.0.0-beta", &server.url("/jei-beta.jar"), &beta)
                    .channel("beta")
                    .published("2026-06-01T00:00:00Z"),
            ),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["jei"]),
        MC,
        LOADER,
    )
    .await
    .unwrap();

    assert_eq!(plan.mods[0].candidate.version_number, "19.0.0");
}

/// Two mods that claim each other without the search converging: the pass
/// limit exists for this, and its message must say what's happening rather
/// than let it spin.
#[tokio::test]
async fn a_pack_that_never_stabilizes_eventually_stops() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("unstable");

    // Each project supplies a `modId` different from the one it requires,
    // and the missing one's slug exists: the queue refills every pass
    // without ever closing the gap.
    for (rank, next) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7)] {
        let slug = format!("chain{rank}");
        let content = jar(
            &format!("supplies{rank}"),
            &[(&format!("chain{next}"), "BOTH")],
        );
        let route = format!("/{slug}.jar");
        server.bytes(&route, &content);
        publish(
            &server,
            &Project::new(&slug).version(Version::new("1.0", &server.url(&route), &content)),
        );
    }

    let error = resolve(
        &registry(&workshop, &server),
        &requests(&["chain0"]),
        MC,
        LOADER,
    )
    .await
    .expect_err("the chain never closes");

    assert!(
        format!("{error:#}").contains("doesn't stabilize"),
        "{error:#}"
    );
}

/// The case that got Sodium, Iris and EntityCulling pulled from the samflix
/// pack.
///
/// Two distinct mods bundle the same library — the Fabric shims for Sodium
/// and Iris, tr7zw's libs for EntityCulling and Not Enough Animations. A
/// bundled `modId` is not the mod's identity: NeoForge knows how to
/// deduplicate bundled jars at load time, and a pack that keeps Iris
/// without Sodium is broken.
#[tokio::test]
async fn two_mods_bundling_the_same_library_both_survive() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("shared-bundled");

    for slug in ["sodium", "iris"] {
        let content = jar_with_bundled(slug, "fabric_api_base");
        let route = format!("/{slug}.jar");
        server.bytes(&route, &content);
        publish(
            &server,
            &Project::new(slug).version(Version::new("1.0", &server.url(&route), &content)),
        );
    }

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["sodium", "iris"]),
        MC,
        LOADER,
    )
    .await
    .expect("both mods are requested in the manifest");

    assert_eq!(retained(&plan), vec!["iris", "sodium"], "a mod disappeared");
}

/// Two jars that really carry the same root `modId` still get deduplicated:
/// the original rule still holds, it was its scope that was too broad. Here
/// both are explicit, so the manifest's contradiction gets announced.
#[tokio::test]
async fn two_explicit_mods_with_the_same_root_modid_stop_resolution() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("same-root-modid");

    // Two distinct projects publish the same mod — the case of a fork, or a
    // mirror. NeoForge would only load one.
    for slug in ["jade", "jade-mirror"] {
        let content = jar("jade", &[]);
        let route = format!("/{slug}.jar");
        server.bytes(&route, &content);
        publish(
            &server,
            &Project::new(slug).version(Version::new("1.0", &server.url(&route), &content)),
        );
    }

    let error = resolve(
        &registry(&workshop, &server),
        &requests(&["jade", "jade-mirror"]),
        MC,
        LOADER,
    )
    .await
    .expect_err("the manifest requests the same mod twice");

    // The discarded mod, the one keeping the spot, and the modId at fault
    // must all be named: without them, the message teaches nothing.
    let text = format!("{error:#}");
    assert!(text.contains("jade-mirror"), "{text}");
    assert!(text.contains("modId"), "{text}");
}

/// A dependency discarded in favor of a mod carrying the same root `modId`
/// is not a contradiction: nobody had named it explicitly. Resolution
/// continues, and says so.
#[tokio::test]
async fn a_dependency_discarded_by_deduplication_stops_nothing() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("dedup-dependency");

    // `main` requires `jade`, which two projects supply under the same root
    // modId; one is requested in the manifest, the other arrives as a
    // declared dependency.
    let jade = jar("jade", &[]);
    server.bytes("/jade.jar", &jade);
    publish(
        &server,
        &Project::new("jade").version(Version::new("1.0", &server.url("/jade.jar"), &jade)),
    );

    let mirror = jar("jade", &[]);
    server.bytes("/jade-mirror.jar", &mirror);
    publish(
        &server,
        &Project::new("jade-mirror").version(Version::new(
            "1.0",
            &server.url("/jade-mirror.jar"),
            &mirror,
        )),
    );

    let main = jar("main", &[]);
    server.bytes("/main.jar", &main);
    publish(
        &server,
        &Project::new("main")
            .version(Version::new("1.0", &server.url("/main.jar"), &main).declare("jade-mirror")),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["jade", "main"]),
        MC,
        LOADER,
    )
    .await
    .expect("a discarded dependency doesn't stop the install");

    assert_eq!(retained(&plan), vec!["jade", "main"]);
}

/// The second collision found on the pack: EntityCulling and Not Enough
/// Animations both bundle `transition` and `trender`, tr7zw's libs. Two
/// shared libraries and not just one — the faulty deduplication stopped at
/// the first one encountered, but the result was the same.
#[tokio::test]
async fn two_shared_libraries_are_no_more_a_duplicate() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("tr7zw");

    for slug in ["entityculling", "not-enough-animations"] {
        let content = jar_with_two_bundled(slug, "transition", "trender");
        let route = format!("/{slug}.jar");
        server.bytes(&route, &content);
        publish(
            &server,
            &Project::new(slug).version(Version::new("1.0", &server.url(&route), &content)),
        );
    }

    let plan = resolve(
        &registry(&workshop, &server),
        &requests(&["entityculling", "not-enough-animations"]),
        MC,
        LOADER,
    )
    .await
    .expect("both mods are requested in the manifest");

    assert_eq!(
        retained(&plan),
        vec!["entityculling", "not-enough-animations"],
        "a mod disappeared"
    );
}

/// The Iris/Sodium scenario, end to end.
///
/// The manifest requests "sodium" with no version — it has no opinion on
/// which one — and "iris", which declares a dependency on a **precise**
/// Sodium build. The old rule gave Sodium the latest version because the
/// manifest always prevailed; Iris's mixins then applied to classes that no
/// longer existed, and Minecraft crashed on the first connection.
///
/// The request that specifies now wins over the one that says nothing.
#[cfg(unix)]
#[tokio::test]
async fn a_pinned_dependency_imposes_its_version_on_an_unversioned_request() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("dependency-pin");

    let old = jar("sodium", &[]);
    let recent = jar("sodium", &[]);
    server.bytes("/sodium-0.6.jar", &old);
    server.bytes("/sodium-0.8.jar", &recent);
    publish(
        &server,
        &Project::new("sodium")
            .version(
                Version::new("0.6", &server.url("/sodium-0.6.jar"), &old)
                    .published("2025-04-04T00:00:00Z"),
            )
            .version(
                Version::new("0.8", &server.url("/sodium-0.8.jar"), &recent)
                    .published("2026-08-28T00:00:00Z"),
            ),
    );

    let iris = jar("iris", &[]);
    server.bytes("/iris.jar", &iris);
    publish(
        &server,
        &Project::new("iris").version(
            Version::new("1.8.12", &server.url("/iris.jar"), &iris).declare_build("sodium", "0.6"),
        ),
    );

    let plan = resolve(
        &registry(&workshop, &server),
        &[Request::new("sodium"), Request::new("iris")],
        MC,
        LOADER,
    )
    .await
    .expect("both mods resolve");

    let sodium = plan
        .mods
        .iter()
        .find(|m| m.candidate.slug == "sodium")
        .expect("sodium is in the pack");

    assert_eq!(
        sodium.candidate.version_number, "0.6",
        "Iris's pinned dependency should have won over an unversioned request"
    );
}

/// And the reverse: when the manifest pins too, it's the one that decides.
/// It stays sovereign as soon as it says something.
#[cfg(unix)]
#[tokio::test]
async fn a_manifest_that_pins_keeps_the_last_word() {
    let server = mc_testkit::Server::new().await;
    let workshop = Workshop::new("manifest-pin");

    let old = jar("sodium", &[]);
    let recent = jar("sodium", &[]);
    server.bytes("/sodium-0.6.jar", &old);
    server.bytes("/sodium-0.8.jar", &recent);
    publish(
        &server,
        &Project::new("sodium")
            .version(Version::new("0.6", &server.url("/sodium-0.6.jar"), &old))
            .version(Version::new("0.8", &server.url("/sodium-0.8.jar"), &recent)),
    );

    let iris = jar("iris", &[]);
    server.bytes("/iris.jar", &iris);
    publish(
        &server,
        &Project::new("iris").version(
            Version::new("1.8.12", &server.url("/iris.jar"), &iris).declare_build("sodium", "0.6"),
        ),
    );

    let mut pinned = Request::new("sodium");
    pinned.file = Some("sodium-0.8".into());

    let plan = resolve(
        &registry(&workshop, &server),
        &[pinned, Request::new("iris")],
        MC,
        LOADER,
    )
    .await
    .expect("both mods resolve");

    let sodium = plan
        .mods
        .iter()
        .find(|m| m.candidate.slug == "sodium")
        .expect("sodium is in the pack");

    assert_eq!(
        sodium.candidate.version_number, "0.8",
        "a manifest that pins must keep the last word"
    );
}
