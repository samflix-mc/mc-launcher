//! What a download makes known while it's working.
//!
//! The launcher brings down close to a gigabyte spread across a few
//! thousand files. Saying nothing about it gives a frozen window for several
//! minutes, and a player who can't tell "it's working" from "it's crashed".
//! The final report is useless: it arrives once the wait is over.
//!
//! This module decides nothing and displays nothing. It opens a channel:
//! [`Downloader`](crate::Downloader) emits [`Progress`] events, someone else
//! aggregates and shows them.
//!
//! ## Why deltas and not a running total
//!
//! [`Progress::Received`] carries what just arrived, never a total. Downloads
//! are concurrent — sixteen assets in flight at once — and a per-file total
//! would force the observer to keep per-file state to reconstruct it. Deltas
//! add up without knowing anything about who sends them.
//!
//! That's also what makes [`Progress::Lost`] necessary: an attempt that fails
//! partway through a body has already had its bytes counted, and the retry
//! will count them again. Without an explicit subtraction, a flaky source
//! would push the bar past 130%.

use std::sync::Arc;

use crate::Fetched;

/// A download event.
///
/// The lifetimes are borrowed: the observer gets the file name for the
/// duration of the call and copies it if it needs to keep it. Copying it
/// systematically would cost one allocation per chunk received, i.e. tens of
/// thousands for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress<'a> {
    /// A batch starts, and its weight is already known.
    ///
    /// Without this, no time remaining is computable: the total would only
    /// exist by adding up `Content-Length`s as they come in, which gives a
    /// total that grows and a bar that moves backward. The three big
    /// batches — libraries, assets, mods — know their sizes before starting,
    /// and this is where they announce them. `bytes` is zero when the source
    /// doesn't publish them.
    Batch { files: usize, bytes: u64 },
    /// A file starts coming down.
    Started { file: &'a str, bytes: Option<u64> },
    /// Bytes just arrived from the network. A delta, never a running total.
    Received(u64),
    /// An attempt failed partway through the body: what it had counted must
    /// be subtracted before the retry counts it again.
    Lost(u64),
    /// A file is settled — downloaded, or already matching on disk.
    ///
    /// `bytes` is what it weighs, not what transited: an already-present
    /// file made nothing come down and yet must move the bar forward,
    /// without which a reinstall would stay at zero until the very end.
    /// That's what lets the observer tell the two apart without adding them
    /// twice — the network is counted through `Received`, the disk through
    /// these `Finished` events.
    Finished {
        file: &'a str,
        state: Fetched,
        bytes: u64,
    },
}

/// Who's listening.
///
/// `Send + Sync` because downloads are concurrent: several tokio tasks call
/// the observer at the same time, with no lock on our part. It's up to it to
/// count atomically.
pub type Observer = Arc<dyn for<'a> Fn(Progress<'a>) + Send + Sync>;

#[cfg(test)]
#[path = "progress.test.rs"]
mod tests;
