//! Where the value comes from: launch, compilation, or the default.

use super::Environment;

/// Value frozen into the binary at compile time, set by the CI.
const COMPILED: Option<&str> = option_env!("SAMFLIX_ENV");

/// Environment of this run.
pub fn current() -> Environment {
    resolve(std::env::var("SAMFLIX_ENV").ok().as_deref(), COMPILED)
}

/// Where was the environment decided? Shown by diagnostics, so an
/// unexpected environment can be traced to its source in one read.
pub fn origin() -> &'static str {
    match (
        std::env::var("SAMFLIX_ENV")
            .ok()
            .as_deref()
            .and_then(Environment::parse),
        COMPILED.and_then(Environment::parse),
    ) {
        (Some(_), _) => "SAMFLIX_ENV variable at launch",
        (None, Some(_)) => "SAMFLIX_ENV frozen at compile time",
        (None, None) => "default, no declaration",
    }
}

/// Resolution kept separate from reading the environment, to be testable.
fn resolve(runtime: Option<&str>, compiled: Option<&str>) -> Environment {
    runtime
        .and_then(Environment::parse)
        .or_else(|| compiled.and_then(Environment::parse))
        .unwrap_or(Environment::Local)
}

#[cfg(test)]
#[path = "resolution.test.rs"]
mod tests;
