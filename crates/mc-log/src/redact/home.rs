//! The home directory, which carries an account name.

/// Replaces the home directory with `~`.
///
/// An absolute path carries the user's account name. That's not a
/// secret, but it's personal data that adds nothing over the relative
/// path.
pub(super) fn redact_home(text: &str) -> String {
    match HOME.as_deref() {
        Some(home) if text.contains(home) => text.replace(home, "~"),
        _ => text.to_string(),
    }
}

/// The home directory, read once.
///
/// `redact` sees every line written to the log: re-reading the
/// environment on each one would take its global lock — which other
/// threads write to elsewhere — for a value that doesn't change for the
/// life of the process. The root "/" is rejected: it prefixes
/// everything.
static HOME: std::sync::LazyLock<Option<String>> = std::sync::LazyLock::new(|| {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    usable_home(&home.to_string_lossy())
});

/// What we're willing to replace with `~`, read once — hence this
/// function, the only way to exercise the two values that must be
/// rejected without relaunching the process with a different
/// environment.
fn usable_home(home: &str) -> Option<String> {
    (!home.is_empty() && home != "/").then(|| home.to_string())
}

#[cfg(test)]
#[path = "home.test.rs"]
mod tests;
