//! The placement, exercised in its own process.
//!
//! `place` writes to a `OnceLock`: there's only one placement per process,
//! and the tests in a single binary run in parallel across threads. A second
//! test that placed its own would fail or succeed depending on the
//! scheduler.
//!
//! Hence this separate binary, which holds only ONE test.

use std::path::PathBuf;

use mc_paths::{Locations, current, from_bases, place, placed};

/// What this test kills, and what no other test could kill: the mutant that
/// replaces the body of `current()` with `from_system()`.
///
/// For that, the placed roots must be IMPOSSIBLE to mistake for the
/// system's. Hence `/tmp/mc-paths-<pid>/…`: an artificial path that no
/// platform convention would produce. Placing the same roots as the system
/// would let the test pass with the mutant in place, and the survivor would
/// live on indefinitely.
#[test]
fn a_placement_takes_hold_then_is_never_replaced() {
    let artificial = PathBuf::from(format!("/tmp/mc-paths-{}", std::process::id()));

    assert!(
        !placed(),
        "nothing should have been placed before this test"
    );
    let before = current();

    let wanted = from_bases(mc_paths::Bases {
        data: artificial.join("data"),
        config: artificial.join("config"),
        temporary: artificial.join("tmp"),
    });

    place(wanted.clone()).expect("the first placement succeeds");
    assert!(placed());

    // What we read back is what we placed, not what the environment says.
    assert_eq!(current(), wanted);
    assert_ne!(current(), before, "the placement changed nothing");
    assert!(current().data.starts_with(&artificial));
    // And the derivation also applies to what we impose: the logs stay under
    // the data.
    assert_eq!(current().logs, wanted.data.join("logs"));

    // A second placement is refused, and replaces nothing. It's a
    // sequencing bug that must be seen, not a situation to recover from: two
    // halves of the program on two different trees would be worse than
    // either one alone.
    let others = Locations {
        data: PathBuf::from("/nowhere"),
        config: PathBuf::from("/nowhere"),
        logs: PathBuf::from("/nowhere"),
        temporary: PathBuf::from("/nowhere"),
    };
    place(others).expect_err("the second placement is refused");
    assert_eq!(current(), wanted, "the second placement replaced the first");
}
