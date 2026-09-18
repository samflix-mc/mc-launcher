use super::*;

#[test]
fn the_steps_are_in_the_order_they_occur() {
    // The order isn't cosmetic: it's the order of dependencies. Mojang gives
    // the Java version required, NeoForge needs that Java, the mods need the
    // loader in place. An interface that showed the path in a different order
    // would lie about what's left to do.
    let ranks: Vec<usize> = Step::ALL.iter().map(|step| step.rank()).collect();

    assert_eq!(ranks, (0..Step::ALL.len()).collect::<Vec<_>>());
}

#[test]
fn each_step_has_a_distinct_identifier() {
    // These strings cross the bridge to the interface and serve as a key.
    // Two steps sharing the same one would light up only one of them.
    let names: std::collections::BTreeSet<&str> =
        Step::ALL.iter().map(|step| step.as_str()).collect();

    assert_eq!(names.len(), Step::ALL.len());
}

#[test]
fn the_identifiers_do_not_move() {
    // The TypeScript compares against these exact strings. Renaming them
    // breaks the display without breaking a single build.
    assert_eq!(Step::Pack.as_str(), "pack");
    assert_eq!(Step::Loader.as_str(), "loader");
    assert_eq!(Step::Minecraft.as_str(), "minecraft");
    assert_eq!(Step::Java.as_str(), "java");
    assert_eq!(Step::NeoForge.as_str(), "neoforge");
    assert_eq!(Step::Mods.as_str(), "mods");
    assert_eq!(Step::Lock.as_str(), "lock");
}

#[test]
fn the_silent_report_accepts_everything_without_doing_anything() {
    // It serves tests and any caller that only wants the result: what
    // matters is that it doesn't panic on any of the three paths.
    let silent = Silent;
    silent.step(Step::Mods);
    silent.note("something");
    silent.download(mc_dl::Progress::Received(1_024));
}
