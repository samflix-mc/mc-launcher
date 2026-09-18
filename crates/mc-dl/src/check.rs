//! How far to verify a file that's already present.

use crate::Checksum;
use std::path::Path;

/// What [`Downloader::to_file`] did, to distinguish an actual download from
/// an already-matching file in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetched {
    Downloaded,
    AlreadyPresent,
}

/// How to decide that a file that's already present doesn't need to be
/// redownloaded.
///
/// The distinction isn't cosmetic: a Minecraft version's assets weigh more
/// than 800 MB spread across a few thousand objects. Recomputing their SHA-1
/// on every launch rereads the whole disk to almost never find anything.
#[derive(Debug, Clone, Copy)]
pub enum Check<'a> {
    /// Digest recomputed on every pass. For what gets executed — jars,
    /// libraries, runtimes.
    Full(&'a Checksum),
    /// Size as a first barrier, digest checked only on write. For large
    /// volumes of small inert files: a truncated asset has the wrong size,
    /// and a silent corruption at a constant size gives at worst a wrong
    /// texture, never executed code. The exhaustive check stays available on
    /// demand.
    Quick { sum: &'a Checksum, size: u64 },
    /// No digest published, but a size announced. That's all some sources
    /// offer; checking the size beats checking nothing, since an error
    /// response served over HTTP 200 never happens to have the right byte
    /// count.
    Size(u64),
    /// No digest published: only presence can be established.
    Presence,
}

impl Check<'_> {
    pub(crate) fn checksum(&self) -> Option<&Checksum> {
        match self {
            Check::Full(sum) => Some(sum),
            Check::Quick { sum, .. } => Some(sum),
            Check::Size(_) | Check::Presence => None,
        }
    }

    /// Expected size, when the source publishes it.
    pub(crate) fn size(&self) -> Option<u64> {
        match self {
            Check::Quick { size, .. } | Check::Size(size) => Some(*size),
            Check::Full(_) | Check::Presence => None,
        }
    }

    /// Can the file already present be kept without downloading?
    pub(crate) fn accepts_existing(&self, path: &Path) -> bool {
        match self {
            Check::Full(sum) => std::fs::read(path)
                .map(|b| sum.matches(&b))
                .unwrap_or(false),
            Check::Quick { size, .. } | Check::Size(size) => std::fs::metadata(path)
                .map(|m| m.len() == *size)
                .unwrap_or(false),
            Check::Presence => true,
        }
    }
}

#[cfg(test)]
#[path = "check.test.rs"]
mod tests;
