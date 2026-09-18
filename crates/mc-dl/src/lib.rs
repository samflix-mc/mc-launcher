//! Shared plumbing: digest-verified, retryable downloads.
//!
//! Installing a modpack means fetching a few thousand files from five
//! different domains. Three properties are enough to make the operation safe
//! and resumable:
//!
//! - **verified** — every file is checked against the digest published by its
//!   source. A CDN that returns an error page over HTTP 200 is caught here,
//!   not three hours later as a NeoForge crash;
//! - **idempotent** — a file that is already present *and* matching is not
//!   redownloaded. Resuming an interrupted install picks up where it left
//!   off, which matters when there are 2,500 asset objects left;
//! - **atomic** — the write goes through a `.part` file renamed at the end. A
//!   power cut never leaves a truncated file that the next pass's check would
//!   mistake for valid, absent a digest.
//! - **observable** — a gigabyte takes several minutes to come down, and a
//!   window that says nothing the whole time looks no different from a
//!   crashed one. The body is read chunk by chunk, and each chunk is
//!   announced: see [`progress`].
mod check;
mod checksum;
mod download;
pub mod progress;

pub use check::{Check, Fetched};
pub use checksum::{Checksum, sha1_of_file, sha512_of_bytes, sha512_of_file};
pub use download::file::{read_off_thread, write_atomic, write_off_thread};
pub use download::{Absent, Downloader};
pub use progress::{Observer, Progress};

/// Agent announced to every API contacted.
///
/// Modrinth explicitly requires an identifiable agent — `project/version
/// (contact)` — and rate-limits anonymous agents more severely; Mojang and
/// Adoptium don't require it but do log it. One single constant for the
/// whole launcher: that's what makes abuse traceable back to us.
pub const USER_AGENT: &str = "samflix-mc-launcher/0.1 (+https://github.com/samflix-mc/mc-launcher)";
