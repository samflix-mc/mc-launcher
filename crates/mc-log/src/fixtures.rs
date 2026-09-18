//! What this crate's suites share: environment variables.
//!
//! Three of them decide the observed behavior — `SAMFLIX_ENV` for the
//! announced environment, `SAMFLIX_TELEMETRY` and `SENTRY_DSN` for incident
//! reporting — and half a dozen tests set them. They're global to the
//! process: two tests that change them at the same time contradict each
//! other, and one that only reads them fails for another's mistake. Since
//! the 2024 edition, `set_var` in a multi-threaded binary is no longer just
//! a race but undefined behavior.
//!
//! Hence this guard: a single lock for the whole crate, and restoring what
//! was declared before. The CI runs the suite with `SAMFLIX_ENV` set — a
//! test that merely cleared it would change the outcome of the ones that
//! follow, and make the CI depend on execution order.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;

/// Serializes tests that touch PROCESS-WIDE state.
///
/// Environment variables were the first such state, and they name this
/// helper. The Sentry hub is another: `sentry::init` binds a client to the
/// MAIN hub, and `Hub::current()` on any other thread inherits from it — so
/// a test asserting "no client is bound" races any test that binds one,
/// across files, and only when the machine is loaded enough for their
/// windows to overlap. Both take this lock.
///
/// An atomic lock and not a `Mutex`: holding a `MutexGuard` across an
/// `await` is what clippy refuses, and this crate has async suites. It's
/// the same guard mc-pack holds, for the same reason.
static LOCK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Variables {
    /// What was declared before we touched it, by name. `None` for a
    /// variable that didn't exist — clearing it is then the correct way to
    /// restore it.
    initial: RefCell<HashMap<&'static str, Option<OsString>>>,
}

/// Takes the lock without changing anything.
///
/// Worth holding just to *read*, too: a test comparing two reads of the
/// environment would be wrong if a write slipped in between them.
pub(crate) fn variables() -> Variables {
    use std::sync::atomic::Ordering;
    while LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    Variables {
        initial: RefCell::new(HashMap::new()),
    }
}

impl Variables {
    /// Notes what was there, once per variable: it's the value from before
    /// the test that must be restored, not the one from before the last
    /// call.
    fn remember(&self, name: &'static str) {
        self.initial
            .borrow_mut()
            .entry(name)
            .or_insert_with(|| std::env::var_os(name));
    }

    pub(crate) fn set(&self, name: &'static str, value: &str) {
        self.remember(name);
        // SAFETY: the lock guarantees no other test in this binary reads or
        // writes the environment while the guard is alive.
        unsafe {
            std::env::set_var(name, value);
        }
    }

    /// Clears the launch declaration. For `SAMFLIX_ENV`, what remains is
    /// then whatever compilation froze — a case that's worth checking, and
    /// that can't be reached any other way.
    pub(crate) fn unset(&self, name: &'static str) {
        self.remember(name);
        // SAFETY: same lock, same guarantee.
        unsafe {
            std::env::remove_var(name);
        }
    }
}

impl Drop for Variables {
    fn drop(&mut self) {
        for (name, initial) in self.initial.borrow().iter() {
            // SAFETY: the lock is only released after this loop.
            unsafe {
                match initial {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        LOCK.store(false, std::sync::atomic::Ordering::Release);
    }
}
