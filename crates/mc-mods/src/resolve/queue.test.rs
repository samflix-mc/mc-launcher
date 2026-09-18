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
