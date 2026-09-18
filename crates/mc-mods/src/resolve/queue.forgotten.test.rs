//! What the queue removes when a build gives up its place.

use super::{Key, Reason, Request, ResolutionQueue};
use crate::Origin;

fn key(source: Origin, project: &str) -> Key {
    (source, project.to_string())
}

#[test]
fn replacing_a_build_forgets_the_dependencies_of_the_one_it_discards() {
    // Otherwise we install the discarded version's libraries on top of the
    // kept version's, and the lock records them as dependencies of a build
    // that isn't there.
    let mut queue = ResolutionQueue::default();
    let x = key(Origin::Modrinth, "X");
    let y = key(Origin::Modrinth, "Y");
    queue.push(
        Request::new("libA"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );
    queue.push(
        Request::new("libB"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );
    queue.push(
        Request::new("libC"),
        Reason::Declared { by: "Y".into() },
        Some(y),
    );

    queue.forget_dependencies_of(&x);

    // Another requester's stay: Y wasn't replaced.
    let left = queue.next().unwrap();
    assert_eq!(left.request.slug, "libC");
    assert!(queue.is_empty());
}

#[test]
fn forgetting_dependencies_spares_the_namesake_from_the_other_source() {
    // "Jade" is published under the same title on both sources, and both
    // projects coexist until the final deduplication. Removing dependencies
    // by title would sweep away those of its twin too, which nothing else
    // still required: the pack would ship without its library.
    let mut queue = ResolutionQueue::default();
    let modrinth = key(Origin::Modrinth, "nvQzSEkR");
    let curseforge = key(Origin::CurseForge, "324717");
    queue.push(
        Request::new("libA"),
        Reason::Declared { by: "Jade".into() },
        Some(modrinth.clone()),
    );
    queue.push(
        Request::new("libB"),
        Reason::Declared { by: "Jade".into() },
        Some(curseforge),
    );

    queue.forget_dependencies_of(&modrinth);

    let left = queue.next().unwrap();
    assert_eq!(left.request.slug, "libB");
    assert!(queue.is_empty());
}

#[test]
fn forgetting_dependencies_spares_the_manifest() {
    // A manifest mod that shares its name with a replaced parent doesn't
    // have to disappear: its request doesn't come from that parent.
    let mut queue = ResolutionQueue::default();
    let x = key(Origin::Modrinth, "X");
    queue.push(Request::new("libA"), Reason::Explicit, None);
    queue.push(
        Request::new("libA"),
        Reason::Declared { by: "X".into() },
        Some(x.clone()),
    );

    queue.forget_dependencies_of(&x);

    let left = queue.next().unwrap();
    assert_eq!(left.request.slug, "libA");
    assert_eq!(left.reason, Reason::Explicit);
    assert!(queue.is_empty());
}
