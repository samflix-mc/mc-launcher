//! Two branches claim the same project: which one dictates its build?

use crate::Candidate;
use crate::jar::Side;
use crate::resolve::plan::Installed;
use crate::resolve::reason::Reason;

/// What happens to the project already kept when another branch claims it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Arbitration {
    /// The build in place holds. Its side now covers both uses, and
    /// `takes_over` says whether the new requester becomes the one that
    /// answers for its presence — that's the one the lock will name.
    Keep { side: Side, takes_over: bool },
    /// The new build wins: the entry is overwritten, and the discarded
    /// one's dependencies no longer have a requester.
    Replace { side: Side },
}

/// Decides between the build in place and the one showing up.
///
/// Two rules, and nothing else. A less authoritative request never dislodges
/// anyone — that's what makes resolution converge. And two requests naming
/// the same build don't fight over it: the download is the same, only the
/// side changes.
///
/// The side returned is always the union of both: a mod claimed
/// server-side by one branch and client-side by another must end up on both
/// sides, whichever one wins on the version.
pub(super) fn arbitrate(
    incoming: u8,
    kept: u8,
    same_build: bool,
    kept_side: Side,
    incoming_side: Side,
) -> Arbitration {
    let side = kept_side.union(incoming_side);
    if incoming <= kept || same_build {
        Arbitration::Keep {
            side,
            takes_over: incoming > kept,
        }
    } else {
        Arbitration::Replace { side }
    }
}

/// Applies the arbitration to the entry already in place.
///
/// Returns `None` when the build in place holds — there is then nothing
/// left to do for this request —, and the side to record when it must be
/// overwritten.
pub(super) fn confront(
    existing: &mut Installed,
    candidate: &Candidate,
    reason: &Reason,
    incoming: u8,
    side: Side,
    same_build: bool,
) -> Option<Side> {
    match arbitrate(
        incoming,
        existing.authority,
        same_build,
        existing.side,
        side,
    ) {
        Arbitration::Keep { side, takes_over } => {
            existing.side = side;
            if takes_over {
                existing.authority = incoming;
                existing.reason = reason.clone();
            }
            None
        }
        Arbitration::Replace { side } => {
            announce_replacement(candidate, existing, reason);
            Some(side)
        }
    }
}

/// Tells the player which version they will get, and which they won't.
///
/// Staying silent would amount to quietly installing the other branch's
/// build: the pack wouldn't have the version the manifest promises, and
/// nothing would signal it before the first symptom in-game.
/// Out of reach of mutation testing: this function has no effect other than
/// emitting a log line, and Rust offers no stable way to read back its own
/// process's output. What it says, however, is checkable — that's
/// [`replacement_message`], right below.
#[mutants::skip]
pub(super) fn announce_replacement(kept: &Candidate, discarded: &Installed, reason: &Reason) {
    tracing::warn!(
        slug = %kept.slug,
        discarded = %discarded.candidate.version_number,
        kept = %kept.version_number,
        "{}",
        replacement_message(kept, discarded, reason)
    );
}

/// What the warning must say: both versions, and who decided.
///
/// Kept separate from its emission so it can be read back by a test. A
/// message that lost either version would leave the player with "a version
/// was replaced" — which is indistinguishable from silence.
pub(super) fn replacement_message(
    kept: &Candidate,
    discarded: &Installed,
    reason: &Reason,
) -> String {
    format!(
        "\"{}\": {} requires {}, replacing {} kept until now",
        kept.slug,
        reason.describe(),
        kept.version_number,
        discarded.candidate.version_number,
    )
}

#[cfg(test)]
#[path = "arbitration.test.rs"]
mod tests;
