use super::{Channel, DeclaredDep, Origin, Request, complete_digests, dependency};
use crate::resolve::fixtures::candidate;

/// The CurseForge case, which doesn't publish a SHA-512: without this
/// fallback, a jar already verified once would go back out with no digest
/// on the next pass.
#[test]
fn the_lock_fills_in_what_the_source_does_not_publish() {
    let mut c = candidate("jade", "15.10.6");
    let mut request = Request::new("jade");
    request.expected_sha1 = Some("aa".into());
    request.expected_sha512 = Some("bb".into());

    complete_digests(&mut c, &request);

    assert_eq!(c.sha1.as_deref(), Some("aa"));
    assert_eq!(c.sha512.as_deref(), Some("bb"));
}

/// What the source publishes is authoritative: a digest fixed in an older
/// lock must not overwrite the one of the build we just resolved.
#[test]
fn what_the_source_publishes_is_not_overwritten() {
    let mut c = candidate("jade", "15.10.6");
    c.sha512 = Some("the-source's".into());
    let mut request = Request::new("jade");
    request.expected_sha512 = Some("the-lock's".into());

    complete_digests(&mut c, &request);

    assert_eq!(c.sha512.as_deref(), Some("the-source's"));
}

/// A Modrinth identifier doesn't exist on CurseForge: a dependency is
/// looked up in its parent's source, never elsewhere.
#[test]
fn a_dependency_is_looked_up_in_its_parent_s_source() {
    let dep = DeclaredDep {
        project_id: "bookshelf".into(),
        version_id: Some("abc".into()),
    };
    let request = dependency(dep, Origin::CurseForge);

    assert_eq!(request.slug, "bookshelf");
    assert_eq!(request.source, Some(Origin::CurseForge));
    assert_eq!(request.file.as_deref(), Some("abc"));
    // A dependency accepts beta: many libraries only publish in that
    // channel, and refusing it would block the whole pack.
    assert_eq!(request.channel, Some(Channel::Beta));
}
