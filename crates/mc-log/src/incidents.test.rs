use super::*;

#[test]
fn la_telemetrie_se_coupe() {
    // SAFETY : test mono-thread, variable restaurée aussitôt.
    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "0");
    }
    assert!(!telemetry_enabled());
    assert!(dsn().is_none());

    unsafe {
        std::env::set_var("SAMFLIX_TELEMETRY", "1");
    }
    assert!(telemetry_enabled());

    unsafe {
        std::env::remove_var("SAMFLIX_TELEMETRY");
    }
    assert!(telemetry_enabled());
}

#[test]
fn un_dsn_vide_desactive_la_remontee() {
    unsafe {
        std::env::set_var("SENTRY_DSN", "   ");
    }
    assert!(dsn().is_none());
    unsafe {
        std::env::remove_var("SENTRY_DSN");
    }
}
