use super::{EXCERPT_LINES, parse};

/// The excerpt takes five lines before the exception — what the game was
/// doing — and sixty after. A crash file can hold thousands: attaching it
/// all would get the event rejected, attaching none would leave an incident
/// with no trace.
#[test]
fn the_excerpt_frames_the_exception_without_carrying_the_whole_file() {
    let mut lines: Vec<String> = (0..20).map(|i| format!("before-{i:02}")).collect();
    lines.push("java.lang.NullPointerException: nothing".to_string());
    lines.extend((0..200).map(|i| format!("\tat some.place(Thing.java:{i})")));

    let crash = parse(&lines.join("\n")).expect("exception found");
    let excerpt: Vec<&str> = crash.excerpt.lines().collect();

    // Five lines of context, then the exception and what follows it.
    assert_eq!(excerpt.len(), 5 + EXCERPT_LINES);
    assert_eq!(excerpt[0], "before-15");
    assert_eq!(excerpt[5], "java.lang.NullPointerException: nothing");
    // The sixtieth line from the exception, and not one more.
    assert!(
        excerpt.last().unwrap().contains("Thing.java:58"),
        "last line: {:?}",
        excerpt.last()
    );
}

#[test]
fn recognizes_a_module_resolution_exception() {
    // The real case that kept the first launch from succeeding.
    let text = "\
[04:01:48] [main/ERROR] [cpw.mods.modlauncher.Launcher/MODLAUNCHER]: Exception
java.lang.module.ResolutionException: Modules _1._21._1 and minecraft export package com.mojang.blaze3d.systems to module bookshelf
\tat java.base/java.lang.module.Resolver.resolveFail(Unknown Source) ~[?:?]
\tat java.base/java.lang.module.Resolver.failTwoSuppliers(Unknown Source) ~[?:?]";

    let crash = parse(text).expect("exception found");
    assert_eq!(crash.exception, "java.lang.module.ResolutionException");
    assert!(crash.message.starts_with("Modules _1._21._1 and minecraft"));
    assert!(crash.excerpt.contains("resolveFail"));
}

#[test]
fn the_first_exception_wins() {
    // The ones that follow usually stem from the first.
    let text = "\
java.lang.NullPointerException: nothing
\tat some.place(Thing.java:1)
Caused by: java.lang.IllegalStateException: consequence";
    assert_eq!(
        parse(text).unwrap().exception,
        "java.lang.NullPointerException"
    );
}

#[test]
fn a_lone_caused_by_is_recognized() {
    let text = "Caused by: java.io.IOException: disk full";
    let crash = parse(text).unwrap();
    assert_eq!(crash.exception, "java.io.IOException");
    assert_eq!(crash.message, "disk full");
}

#[test]
fn an_ordinary_log_does_not_produce_a_crash() {
    // Without this filter, every successful launch would raise a false incident.
    let text = "\
[04:01:47] [main/INFO] [Launcher/MODLAUNCHER]: ModLauncher running: args [--username, Sam]
[04:01:48] [main/INFO] [ModDiscoverer/SCAN]: Found mod file \"jei-1.21.1.jar\"
[04:01:50] [Render thread/INFO] [minecraft/Minecraft]: Setting user: Sam";
    assert!(parse(text).is_none());
}
