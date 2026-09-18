//! What the launcher answers when asked where it stands, without opening a
//! window.
//!
//! `mc-pack diagnostic` already exists and helps troubleshoot for a player.
//! This one answers a different question, and it's CI that asks it: **is
//! the binary we just published the one we think it is?** A `.deb` built
//! without `SAMFLIX_ENV` compiles, installs, runs — and goes fetch the
//! development pack. Nothing signals it, since the default is silence.
//!
//! Hence a flag and not a subcommand: the binary is a graphical
//! application, not a command-line tool, and it must not gain an argument
//! grammar a player could stumble into by accident.
//!
//! ## Why the published-build check only covers Linux and macOS
//!
//! `main.rs` sets `windows_subsystem = "windows"` in release: the process
//! has no attached console, and `println!` writes to a descriptor that
//! leads nowhere. `AttachConsole(ATTACH_PARENT_PROCESS)` would reattach it,
//! but the calling shell doesn't wait for it and returns before the first
//! line — a CI `grep` would be unstable there depending on whether it's
//! called from cmd.exe or PowerShell. A check that fails one time in three
//! gets disabled within a month, and takes with it the two times out of
//! three it was telling the truth.
//!
//! The flag works under Windows regardless, in development builds, where
//! `windows_subsystem` isn't set.

use std::fmt::Write as _;

/// The flag that triggers the diagnostic instead of the window.
const FLAG: &str = "--diagnostic";

/// Is the diagnostic requested?
///
/// Takes the arguments rather than reading them: that's what makes the
/// decision verifiable without launching a process.
pub fn requested<I, S>(arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    arguments.into_iter().any(|a| a.as_ref() == FLAG)
}

/// What the binary knows about itself, as text.
///
/// Returns a string instead of printing: printing belongs to the caller,
/// and a function that returns its text can be compared in a test.
pub fn report(dmabuf_disabled: bool) -> String {
    let mut text = String::new();

    // The name first: it's the only line that distinguishes two binaries
    // built from the same commit with two different `MC_LAUNCHER_NAME`.
    let _ = writeln!(text, "Launcher");
    let _ = writeln!(text, "  name          : {}", crate::brand::name());
    let _ = writeln!(
        text,
        "  version       : {}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("unknown")
    );

    // The line CI compares against the tag. `origin()` says where the value
    // comes from, which distinguishes "set at build time" from "fell back
    // to the default" — and that's precisely the distinction this check is
    // looking for.
    let _ = writeln!(
        text,
        "  environment   : {} ({})",
        mc_log::environment::current().as_str(),
        mc_log::environment::origin()
    );

    let _ = writeln!(text, "\nRendering");
    let _ = writeln!(
        text,
        "  DMA-BUF       : {}",
        if dmabuf_disabled {
            "disabled (NVIDIA driver detected)"
        } else {
            "left to WebKit"
        }
    );

    // The four roots, and where they come from.
    //
    // `--diagnostic` answers BEFORE the window is built: Tauri's resolver
    // hasn't spoken yet, so what's shown here is what the environment says.
    // It's written in black and white rather than assumed: the two can
    // diverge, and that's precisely the kind of drift a player reports as
    // "it can't find my mods".
    let _ = writeln!(text, "\nLocations (from the environment)");
    let locations = mc_paths::current();
    for (name, path) in locations.list() {
        let _ = writeln!(text, "  {name:<13} : {}", path.display());
    }
    if !mc_paths::placed() {
        let _ = writeln!(text, "  (the application sets Tauri's at window startup)");
    }

    let _ = writeln!(text, "\nLogs and incidents");
    let _ = writeln!(text, "  directory     : {}", mc_log::log_dir().display());
    let _ = writeln!(
        text,
        "  telemetry     : {}",
        if mc_log::telemetry_active() {
            "active — turn off with SAMFLIX_TELEMETRY=0"
        } else {
            "off"
        }
    );

    text
}

#[cfg(test)]
#[path = "diagnostic.test.rs"]
mod tests;
