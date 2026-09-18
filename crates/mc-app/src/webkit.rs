//! What must happen before WebKit initializes.
//!
//! Since WebKitGTK 2.42, rendering goes through DMA-BUF. On the proprietary
//! NVIDIA driver, the buffer exchange fails: the window stays white, or
//! closes as soon as it opens. The known fix is
//! `WEBKIT_DISABLE_DMABUF_RENDERER=1`, and that's an environment variable —
//! something a developer types, that a player never will: they double-click
//! an icon.
//!
//! So it's set here, by the program, for itself.
//!
//! ## Why not all the time
//!
//! Without DMA-BUF, WebKit falls back to a main-memory copy on every frame.
//! On a driver that doesn't have the bug — Intel, AMD, or NVIDIA on
//! `nouveau` — that would be paying a rendering regression for nothing. The
//! variable is only set if the `nvidia` kernel module is loaded.
//!
//! No platform `cfg`: `/sys/module/nvidia` is a Linux path, and it doesn't
//! exist elsewhere. The module is naturally inert on Windows, macOS and
//! Android, with no need to repeat the check there.
//!
//! ## Why it's `unsafe`, and why it's safe here
//!
//! Since the 2024 edition, `set_var` is `unsafe`: writing the environment
//! while another thread reads it is a race. The call is therefore the very
//! first instruction of the process — before `mc_log::init`, which opens a
//! write thread for the log, and well before GTK. It's the only window where
//! the operation is guaranteed to be alone, and that's why
//! [`configure_rendering`] doesn't log: the log doesn't exist yet. It returns
//! what it did, and the caller reports it once `mc-log` is ready.

/// What WebKitGTK reads to know whether it should avoid DMA-BUF.
const VARIABLE: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// The kernel module, present only with the proprietary driver.
const NVIDIA_MODULE: &str = "/sys/module/nvidia";

/// Sets the workaround if the machine needs it. Returns `true` if it did.
///
/// To call first, before anything that might create a thread.
pub fn configure_rendering() -> bool {
    if !should_disable_dmabuf(nvidia_loaded(), already_chosen()) {
        return false;
    }

    // SAFETY: first call of the process. No thread has been created yet —
    // neither the log's, nor GTK's — so nobody reads the environment while
    // we write it.
    unsafe { std::env::set_var(VARIABLE, "1") };
    true
}

/// The decision, isolated from what carries it out.
///
/// `already_chosen` wins either way: whoever sets the variable to `0` has a
/// reason to want DMA-BUF despite NVIDIA — a fixed driver, an experiment —
/// and overwriting it would take away the only way to say so.
fn should_disable_dmabuf(nvidia_loaded: bool, already_chosen: bool) -> bool {
    nvidia_loaded && !already_chosen
}

fn nvidia_loaded() -> bool {
    std::path::Path::new(NVIDIA_MODULE).exists()
}

fn already_chosen() -> bool {
    std::env::var_os(VARIABLE).is_some()
}

#[cfg(test)]
#[path = "webkit.test.rs"]
mod tests;
