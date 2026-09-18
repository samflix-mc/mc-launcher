//! The decision is testable; the driver of the machine running the tests is
//! not.
//!
//! `configure_rendering` reads `/sys/module/nvidia` and writes the process
//! environment: its result depends on the machine, and its effect would leak
//! from one test to another. `should_disable_dmabuf` is what carries the
//! choice, and it takes both its inputs as arguments exactly for that
//! reason.

use super::{NVIDIA_MODULE, VARIABLE, should_disable_dmabuf};

#[test]
fn nvidia_alone_triggers_the_workaround() {
    assert!(should_disable_dmabuf(true, false));
}

#[test]
fn without_nvidia_rendering_stays_intact() {
    // DMA-BUF works elsewhere, and disabling it would cost a copy per frame.
    assert!(!should_disable_dmabuf(false, false));
}

#[test]
fn an_explicit_choice_is_never_overwritten() {
    // Including on NVIDIA: setting the variable to "0" is the only way to
    // reclaim DMA-BUF once the driver has been fixed.
    assert!(!should_disable_dmabuf(true, true));
    assert!(!should_disable_dmabuf(false, true));
}

#[test]
fn the_two_names_do_not_move() {
    // One is read by WebKitGTK, the other by the kernel: a typo doesn't
    // break the build and silently makes the workaround a no-op.
    assert_eq!(VARIABLE, "WEBKIT_DISABLE_DMABUF_RENDERER");
    assert_eq!(NVIDIA_MODULE, "/sys/module/nvidia");
}
