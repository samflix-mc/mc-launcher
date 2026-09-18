//! What keeps the log open for as long as the program runs.

mod log;

use std::path::PathBuf;

pub use log::log_dir;
pub(crate) use log::{current_log_name, purge_old_logs};

/// To be kept alive for as long as the program runs.
///
/// Its destruction flushes the file's write queue, then gives Sentry time
/// to send what's left. Dropping it right away would lose exactly the last
/// messages — the ones describing the exit.
pub struct Guard {
    // The file empties before Sentry waits on the network: the field order
    // is the destruction order. The reverse left the end of the log stuck
    // in the write queue during the ten seconds of sending — and a Ctrl-C
    // during the wait left it there for good.
    _file: Option<tracing_appender::non_blocking::WorkerGuard>,
    _sentry: Option<sentry::ClientInitGuard>,
    /// Log directory and component name, to find the day's file on demand.
    log: Option<(PathBuf, String)>,
}

impl Guard {
    /// Assembled by [`crate::init`] alone, once the layers are in place.
    ///
    /// The argument order matches the field order, and therefore the
    /// destruction order: the file first, Sentry after.
    pub(crate) fn new(
        file: Option<tracing_appender::non_blocking::WorkerGuard>,
        sentry: Option<sentry::ClientInitGuard>,
        log: Option<(PathBuf, String)>,
    ) -> Self {
        Self {
            _file: file,
            _sentry: sentry,
            log,
        }
    }

    /// A guard that holds nothing.
    ///
    /// This is the state of a run whose log directory wasn't writable: the
    /// program runs, but [`log_path`] designates nothing. Exposed because
    /// the binaries receive a `&Guard` argument and this case — the one
    /// where a player must not be pointed to a missing file — can't be
    /// exercised any other way.
    ///
    /// [`log_path`]: Guard::log_path
    pub fn without_log() -> Self {
        Self::new(None, None, None)
    }

    /// A guard that designates a log without holding anything open.
    ///
    /// Complement of the previous one: it makes it possible to exercise both
    /// branches of what [`log_path`] returns, without setting up a global
    /// subscriber — something [`crate::init`] can only do once per process.
    ///
    /// [`log_path`]: Guard::log_path
    #[doc(hidden)]
    pub fn new_for_fixtures(dir: PathBuf, component: &str) -> Self {
        Self::new(None, None, Some((dir, component.to_string())))
    }

    /// Path to the log, to cite when something fails.
    ///
    /// Recomputed on every call, never frozen at startup: `rolling::daily`
    /// switches files at midnight UTC, and a run that started before
    /// continues in the next one. A frozen path would then designate a file
    /// that exists but stops before the failure — more misleading than a
    /// missing file, since nothing invites doubt about it.
    pub fn log_path(&self) -> Option<PathBuf> {
        self.log
            .as_ref()
            .map(|(dir, component)| dir.join(current_log_name(component)))
    }
}

#[cfg(test)]
#[path = "guard.test.rs"]
mod tests;
