//! What turns thousands of events into one readable line.
//!
//! `mc-dl` announces every chunk received — on a full install, that's tens
//! of thousands of events per minute. Passing them straight to the window
//! would drown it: the Tauri bridge serializes each one, and Angular would
//! redraw more often than the screen refreshes, for an unreadable result.
//!
//! Events are therefore **accumulated here**, with no allocation or lock on
//! the hot path, and the full state is emitted at a fixed interval. What
//! reaches the screen is a snapshot taken five times a second, not a
//! stream.
//!
//! ## What moves the bar
//!
//! Two sources, which never add up twice: bytes pulled from the network,
//! and the weight of files already present on disk. Without the second, a
//! reinstall would stay at zero from end to end even though everything's
//! already there; without the first, there would be no rate to display.
//!
//! ## What isn't promised
//!
//! The total comes from the batches announced by the crates. It's exact for
//! Mojang, which publishes sizes, and it's a floor for CurseForge mods
//! without a key, which don't publish any. Remaining time is derived from
//! it: it's an estimate, and the interface presents it as one.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use serde::Serialize;

use crate::phase::Phase;

/// The share of the measured rate that enters the displayed value.
///
/// The instantaneous rate of a batch of small files jumps by a factor of
/// ten from one measurement to the next — sixteen concurrent requests
/// starting and finishing without syncing up. Displayed raw, it flickers and
/// can't be read. A fifth of nine gives a value that takes about a second to
/// catch up with a real change, which is enough to track a network outage
/// without turning the display into a strobe.
const SMOOTHING: f64 = 0.2;

/// The full state, as it goes out to the window.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: Phase,
    /// Is the phase a reached state rather than work in progress?
    ///
    /// "Ready to play" isn't a step being executed: it's the result of the
    /// nine before it. Without this flag, the window would light it up like
    /// a step in progress and suggest there's still something to wait for.
    pub done: bool,
    /// `mc-pack`'s latest note — "128 mods, 43 of them added…".
    pub note: Option<String>,
    /// The file currently downloading.
    pub file: Option<String>,
    /// Bytes acquired: downloaded, plus those already there.
    pub bytes: u64,
    /// What the current batch announced it weighs. Zero when nothing is
    /// announced.
    pub total: u64,
    pub files: usize,
    pub files_total: usize,
    /// Is a download actually in progress?
    ///
    /// False between two batches — while resolving mods, inspecting jars,
    /// running the NeoForge installer. That's where the bar used to stay
    /// stuck at a hundred percent while the step was still working: the
    /// previous batch was done, the next not yet announced. An indeterminate
    /// progress tells the truth, a "100%" lies.
    pub active: bool,
    /// Bytes per second, smoothed.
    pub rate: u64,
    /// Seconds remaining, when the total and the rate allow estimating it.
    pub remaining: Option<u64>,
}

/// The counter shared between downloads and the emission loop.
///
/// The hot fields — the ones a received chunk touches — are atomics: sixteen
/// tasks increment them in parallel, and a lock on that path would cost on
/// every TCP packet. The cold fields, which change a few times a minute, sit
/// behind a `Mutex`.
#[derive(Debug)]
pub struct Tracker {
    phase: Mutex<Phase>,
    done: std::sync::atomic::AtomicBool,
    note: Mutex<Option<String>>,
    file: Mutex<Option<String>>,
    /// Bytes actually downloaded, retries deducted.
    received: AtomicU64,
    /// Weight of the files found conformant on disk.
    already_there: AtomicU64,
    total: AtomicU64,
    files: AtomicUsize,
    files_total: AtomicUsize,
    /// What's needed to compute a rate: what had been acquired at the last
    /// measurement, and when.
    measurement: Mutex<Measurement>,
}

#[derive(Debug)]
struct Measurement {
    acquired: u64,
    instant: Instant,
    rate: f64,
}

impl Default for Tracker {
    fn default() -> Self {
        Self {
            phase: Mutex::new(Phase::SignIn),
            done: std::sync::atomic::AtomicBool::new(false),
            note: Mutex::new(None),
            file: Mutex::new(None),
            received: AtomicU64::new(0),
            already_there: AtomicU64::new(0),
            total: AtomicU64::new(0),
            files: AtomicUsize::new(0),
            files_total: AtomicUsize::new(0),
            measurement: Mutex::new(Measurement {
                acquired: 0,
                instant: Instant::now(),
                rate: 0.0,
            }),
        }
    }
}

impl Tracker {
    /// A phase starts working.
    pub fn phase(&self, phase: Phase) {
        *self.phase.lock().expect("phase") = phase;
        self.done.store(false, Ordering::Relaxed);
    }

    /// Work stops on this phase, which is a state and not an action.
    ///
    /// Clears the batch along the way: without this, the last download's bar
    /// would stay displayed full under "Ready to play", as if something were
    /// still going on.
    pub fn finish(&self, phase: Phase) {
        *self.phase.lock().expect("phase") = phase;
        self.done.store(true, Ordering::Relaxed);
        *self.file.lock().expect("file") = None;
        self.forget_the_batch();
    }

    /// Mod resolution advances: so many requests settled out of so many.
    ///
    /// **It downloads almost nothing, and that's the problem it fixes.**
    /// Querying the APIs for a fifty-mod pack takes about thirty seconds for
    /// a few dozen kilobytes: the byte bar wouldn't move, the rate would
    /// stay at zero, and the screen would look no different from a crash.
    /// The request count, though, moves.
    ///
    /// The byte total is reset to zero: that's what tells the window to
    /// rely on the COUNT rather than the weight, for the time being. The
    /// download batch that follows will announce it again.
    pub fn resolution(&self, done: usize, total: usize) {
        self.files.store(done, Ordering::Relaxed);
        self.files_total.store(total, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
    }

    pub fn note(&self, text: &str) {
        *self.note.lock().expect("note") = Some(text.to_string());
    }

    /// Records a download event.
    ///
    /// Called from `mc-dl`'s concurrent tasks, tens of thousands of times:
    /// everything done here is lock-free, except for the file name, which
    /// only changes once per file.
    pub fn download(&self, progress: mc_dl::Progress<'_>) {
        match progress {
            mc_dl::Progress::Batch { files, bytes } => {
                // A batch replaces the previous one rather than adding to
                // it: steps chain, and the bar restarts from zero at each
                // one — which the phase path already explains.
                self.received.store(0, Ordering::Relaxed);
                self.already_there.store(0, Ordering::Relaxed);
                self.files.store(0, Ordering::Relaxed);
                self.total.store(bytes, Ordering::Relaxed);
                self.files_total.store(files, Ordering::Relaxed);
            }
            mc_dl::Progress::Started { file, .. } => {
                *self.file.lock().expect("file") = Some(file.to_string());
            }
            mc_dl::Progress::Received(bytes) => {
                self.received.fetch_add(bytes, Ordering::Relaxed);
            }
            mc_dl::Progress::Lost(bytes) => {
                // `saturating_sub` on an atomic counter is written like
                // this: subtracting more than what's counted would wrap
                // back around zero and display sixteen exabytes.
                let _ =
                    self.received
                        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |acquired| {
                            Some(acquired.saturating_sub(bytes))
                        });
            }
            mc_dl::Progress::Finished { state, bytes, .. } => {
                self.files.fetch_add(1, Ordering::Relaxed);
                if state == mc_dl::Fetched::AlreadyPresent {
                    self.already_there.fetch_add(bytes, Ordering::Relaxed);
                }
            }
        }
    }

    /// The full state, and the rate measured since the last call.
    ///
    /// The rate is computed here rather than per chunk because it only makes
    /// sense over an interval: it's the emission loop that fixes it, by
    /// calling at a regular cadence.
    pub fn snapshot(&self) -> Progress {
        let acquired =
            self.received.load(Ordering::Relaxed) + self.already_there.load(Ordering::Relaxed);
        let total = self.total.load(Ordering::Relaxed);

        let rate = {
            let mut measurement = self.measurement.lock().expect("measurement");
            let elapsed = measurement.instant.elapsed().as_secs_f64();
            let instant_rate = instant_rate(acquired.saturating_sub(measurement.acquired), elapsed);
            measurement.rate = smooth(measurement.rate, instant_rate);
            measurement.acquired = acquired;
            measurement.instant = Instant::now();
            measurement.rate
        };

        let files = self.files.load(Ordering::Relaxed);
        let files_total = self.files_total.load(Ordering::Relaxed);

        Progress {
            phase: *self.phase.lock().expect("phase"),
            done: self.done.load(Ordering::Relaxed),
            note: self.note.lock().expect("note").clone(),
            file: self.file.lock().expect("file").clone(),
            bytes: acquired,
            total,
            files,
            files_total,
            active: in_progress(files, files_total),
            rate: rate as u64,
            remaining: remaining(total, acquired, rate),
        }
    }

    /// Forgets the current batch: nothing left to show of a download.
    fn forget_the_batch(&self) {
        self.received.store(0, Ordering::Relaxed);
        self.already_there.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.files.store(0, Ordering::Relaxed);
        self.files_total.store(0, Ordering::Relaxed);
    }
}

/// Are there still files to settle in the announced batch?
///
/// The answer decides what the window draws: a numbered bar, or
/// indeterminate progress. A complete batch but a step still working — mod
/// resolution between two passes, the NeoForge installer running in its
/// JVM — is exactly the case where a full bar would lie.
fn in_progress(files: usize, total: usize) -> bool {
    total > 0 && files < total
}

/// Bytes per second over the interval that just elapsed.
///
/// A null interval — two calls in the same microsecond — would yield
/// infinity; we return zero, which smoothing absorbs without making the
/// display flicker.
fn instant_rate(bytes: u64, elapsed: f64) -> f64 {
    if elapsed <= 0.0 {
        return 0.0;
    }
    bytes as f64 / elapsed
}

/// Exponential moving average.
fn smooth(previous: f64, measurement: f64) -> f64 {
    previous * (1.0 - SMOOTHING) + measurement * SMOOTHING
}

/// Seconds remaining, when the question makes sense.
///
/// Three cases return `None`, and none of them is an error: no total
/// announced (the source doesn't publish sizes), nothing still downloading
/// (the rate is zero, dividing would give infinity), and a total already
/// exceeded — which happens when mods without a published size counted for
/// zero. Displaying "0s" in that last case would suggest it's finished.
fn remaining(total: u64, acquired: u64, rate: f64) -> Option<u64> {
    if total == 0 || rate <= 0.0 || acquired >= total {
        return None;
    }
    Some(((total - acquired) as f64 / rate).ceil() as u64)
}

#[cfg(test)]
#[path = "tracker.test.rs"]
mod tests;
