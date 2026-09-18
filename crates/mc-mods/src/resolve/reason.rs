//! Why a mod ends up in the pack, and what weight that gives it.

use super::request::Request;

/// or whether it can be removed once its parent is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// Named in the manifest.
    Explicit,
    /// Dependency declared by the source's API.
    Declared { by: String },
    /// Dependency that no API announced, read from a jar's descriptor.
    Implicit { by: String, mod_id: String },
}

impl Reason {
    pub fn describe(&self) -> String {
        match self {
            Reason::Explicit => "requested by the manifest".to_string(),
            Reason::Declared { by } => format!("declared dependency of {by}"),
            Reason::Implicit { by, mod_id } => {
                format!("implicit dependency: {by} requires \"{mod_id}\"")
            }
        }
    }
}

/// What settles a dispute between two branches claiming the same project.
///
/// Two criteria, and **pinning outranks origin**:
///
/// | | pinned | open |
/// |---|---|---|
/// | manifest | 12 | 4 |
/// | declared dependency | 10 | 2 |
/// | requirement read from a jar | 8 | 0 |
///
/// Without this order, whoever arrived first kept the spot — so the random
/// order of traversal decided which version got installed.
///
/// ## Why pinning outranks origin
///
/// The manifest long prevailed under all circumstances, even without a
/// version. That was a blind spot: **a request without a version expresses
/// no version preference**. Writing "sodium" says "I want this mod," not "I
/// want its latest version whatever the cost."
///
/// The case that exposed it: a pack requested "sodium" without a version,
/// Iris declared a dependency on a specific Sodium build — its compatibility
/// mixins target classes that change name from one version to the next. The
/// old rule gave Sodium the latest version, Iris's mixins applied to
/// nothing, and Minecraft crashed on the first connection on a missing
/// class.
///
/// The manifest stays sovereign **as soon as it says something**: a request
/// it pins beats everything else. It's the same rule as cargo or npm — a
/// strict constraint outranks "any version."
pub(super) fn authority(reason: &Reason, request: &Request) -> u8 {
    let origin = match reason {
        Reason::Explicit => 4,
        Reason::Declared { .. } => 2,
        Reason::Implicit { .. } => 0,
    };
    // Eight: more than the maximum gap between two origins, so no origin
    // ever catches up to a pin.
    let pinned = request.file.is_some() || request.version.is_some();
    origin + if pinned { 8 } else { 0 }
}

/// Has a requirement read from a jar just lost its spot for good?
///
/// True when the request is implicit, when a requester at least as
/// authoritative already holds the key, and it holds a different build. The
/// next pass would raise the same gap, propose the same project again and
/// lose it again identically: the queue would never empty and resolution
/// would die on [`MAX_PASSES`], blaming circular dependencies that don't
/// exist. The gap is recorded once, and the install continues.
pub(super) fn implicit_impasse(reason: &Reason, incoming: u8, kept: u8, same_build: bool) -> bool {
    matches!(reason, Reason::Implicit { .. }) && incoming <= kept && !same_build
}

#[cfg(test)]
#[path = "reason.test.rs"]
mod tests;
