use super::{Key, Reason, Request, ResolutionQueue};
use crate::Origin;

fn key(source: Origin, project: &str) -> Key {
    (source, project.to_string())
}

#[test]
fn the_manifest_goes_before_dependencies_even_when_pushed_along_the_way() {
    // The original defect lived entirely here. With a single stack, the
    // dependency pushed by "b" would jump ahead of the remaining "a"
    // request: "a" was resolved without its pin, and the pinned request
    // then found the key already taken. The player would install a
    // different build than the one in the lock, with nothing to signal it.
    let mut queue = ResolutionQueue::default();
    queue.push(Request::new("a"), Reason::Explicit, None);
    queue.push(Request::new("b"), Reason::Explicit, None);

    let first = queue.next().unwrap();
    assert_eq!(first.request.slug, "b");
    queue.push(
        Request::new("a"),
        Reason::Declared { by: "b".into() },
        Some(key(Origin::Modrinth, "b")),
    );

    let then = queue.next().unwrap();
    assert_eq!(then.request.slug, "a");
    assert_eq!(then.reason, Reason::Explicit);

    let last = queue.next().unwrap();
    assert_eq!(last.request.slug, "a");
    assert_eq!(last.reason, Reason::Declared { by: "b".into() });
    assert!(queue.is_empty());
}

/// **`remaining` counts BOTH queues, and that is the whole point.**
///
/// It feeds the total the progress bar announces — `pass.rs` calls it twice
/// to say "x out of y mods". A version returning only the manifest's count,
/// or a constant, would announce a total that doesn't move while the
/// resolution spawns dependency after dependency: the bar would sit at
/// "0 of 0" through the thirty-six seconds the API polling can take, which
/// is exactly the frozen window this counter exists to prevent.
///
/// Nothing else observes it, so nothing else would catch that. This test is
/// written after a mutant survived here: `remaining -> usize with 0` passed
/// the whole suite.
#[test]
fn remaining_counts_the_manifest_and_the_dependencies_together() {
    let mut queue = ResolutionQueue::default();
    assert_eq!(queue.remaining(), 0);

    queue.push(Request::new("a"), Reason::Explicit, None);
    queue.push(Request::new("b"), Reason::Explicit, None);
    assert_eq!(queue.remaining(), 2);

    // A dependency lands in the OTHER queue: the count must follow it there
    // too, which is what a manifest-only count would miss.
    queue.push(
        Request::new("c"),
        Reason::Declared { by: "b".into() },
        Some(key(Origin::Modrinth, "b")),
    );
    assert_eq!(queue.remaining(), 3);

    queue.next().unwrap();
    assert_eq!(queue.remaining(), 2);

    queue.next().unwrap();
    queue.next().unwrap();
    assert_eq!(queue.remaining(), 0);
    assert!(queue.is_empty());
}
