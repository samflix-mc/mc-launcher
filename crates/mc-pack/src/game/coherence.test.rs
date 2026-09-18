use super::verify;
use crate::fixtures::{JAR, Workshop, entry, lock};

/// The lock comes from this pack's cache; the instance carries the pack's
/// name and there's only one for all three environments. An `install` run
/// with a different `SAMFLIX_ENV` could therefore have replaced these jars
/// without this lock knowing anything about it.
#[test]
fn an_instance_installed_from_another_pack_is_refused() {
    let workshop = Workshop::new("coherence-mismatch");
    workshop.installed_pack(vec![entry("jei", "both", None)]);
    let instance = workshop.options().layout.instance("samflix");

    // The lock we read announces a mod the instance doesn't have.
    let lock = lock(vec![
        entry("jei", "both", None),
        entry("sodium", "client", None),
    ]);

    let error = verify(&lock, &instance).expect_err("the instance diverges");
    let text = format!("{error:#}");
    assert!(text.contains("sodium.jar"), "{text}");
    assert!(text.contains("mc-pack install"), "{text}");
    // The environment and its origin are stated: that's what lets someone
    // understand why the two diverge.
    assert!(text.contains("environment"), "{text}");
}

#[test]
fn a_conforming_instance_passes() {
    let workshop = Workshop::new("coherence-ok");
    workshop.installed_pack(vec![entry("jei", "both", None)]);
    let instance = workshop.options().layout.instance("samflix");

    verify(&lock(vec![entry("jei", "both", None)]), &instance).expect("everything is there");
}

/// A server mod missing from the client folder is not a divergence: it has
/// no business being there.
#[test]
fn a_server_mod_does_not_count_client_side() {
    let workshop = Workshop::new("coherence-server");
    workshop.installed_pack(Vec::new());
    let instance = workshop.options().layout.instance("samflix");
    std::fs::write(instance.mods_dir().join("jei.jar"), JAR).unwrap();

    let lock = lock(vec![
        entry("jei", "client", None),
        entry("ledger", "server", None),
    ]);
    verify(&lock, &instance).expect("only the client side counts here");
}
