//! Makes `SAMFLIX_ENV` visible at compile time, and recompiles when it changes.
//!
//! `option_env!` is evaluated once, at compile time. Without this directive,
//! cargo would keep a binary carrying the old value cached: a preproduction
//! build re-run with `SAMFLIX_ENV=production` would keep announcing
//! "preproduction", and the dashboard would lie without anything signaling it.
//!
//! ## The guard, and why it's here
//!
//! A missing `SAMFLIX_ENV` doesn't break anything: resolution falls back to
//! "local", the cautious default. That's exactly what makes the omission
//! dangerous in a release — the binary ships, installs, launches, and goes
//! looking for the development pack believing it's doing the right thing.
//! Nothing says so, since the default is silence.
//!
//! `MC_EXIGER_ENV=1` reverses that default: wherever it's set — the four
//! Tauri legs of release.yml — a missing environment stops the build instead
//! of letting it produce a binary that lies. It's set nowhere else: on a
//! development machine, compiling without thinking about it must stay
//! possible.
//!
//! The guard lives in this build.rs rather than in an `if` in the workflow
//! because this is where we know what the binary will actually carry — a
//! workflow only sees a variable, not the result of its propagation through
//! a `uses:`.

fn main() {
    println!("cargo:rerun-if-env-changed=SAMFLIX_ENV");
    // On its own line, and that's the detail without which the guard would
    // miss the build it was supposed to stop: cargo only reruns a build
    // script if one of the DECLARED variables changed. Setting MC_EXIGER_ENV
    // without declaring it would leave a cached script — from a previous
    // build, one that checked nothing — deciding in its place.
    println!("cargo:rerun-if-env-changed=MC_EXIGER_ENV");

    let required = std::env::var("MC_EXIGER_ENV")
        .map(|value| !value.trim().is_empty() && value != "0")
        .unwrap_or(false);

    if !required {
        return;
    }

    let declared = std::env::var("SAMFLIX_ENV")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if !declared {
        // `cargo::error` and not a panic: the message comes out formatted as
        // a compile error, in its place in the job log, instead of a panic
        // trace that reads like a tooling bug.
        println!(
            "cargo::error=MC_EXIGER_ENV is set but SAMFLIX_ENV is not. \
             The produced binary would believe itself \"local\": it would go \
             looking for the development pack and let players onto the dev \
             server. Set SAMFLIX_ENV, or remove MC_EXIGER_ENV if this build \
             isn't meant to be published."
        );
    }
}
