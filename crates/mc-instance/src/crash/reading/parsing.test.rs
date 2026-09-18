use super::{split_exception, strip_ansi};

#[test]
fn a_timestamp_is_not_mistaken_for_an_exception() {
    // "04:01:47" contains colons and digits.
    assert!(split_exception("[04:01:47] [main/INFO] startup").is_none());
}

#[test]
fn a_file_path_is_not_mistaken_for_an_exception() {
    assert!(split_exception("file: /home/sam/.local/share/thing.jar").is_none());
}

#[test]
fn a_trace_line_is_not_a_declaration() {
    assert!(split_exception("\tat java.base/java.lang.Thread.run(Thread.java:1)").is_none());
}

#[test]
fn colors_are_stripped() {
    let colored = "\u{1b}[32m[04:01] ERROR\u{1b}[m next";
    assert_eq!(strip_ansi(colored), "[04:01] ERROR next");
}

#[test]
fn a_colored_exception_stays_recognizable() {
    // Minecraft colors its output; without stripping, nothing matches.
    let line = "\u{1b}[31mjava.lang.OutOfMemoryError: Java heap space\u{1b}[m";
    let (exception, message) = split_exception(line).unwrap();
    assert_eq!(exception, "java.lang.OutOfMemoryError");
    assert_eq!(message, "Java heap space");
}

#[test]
fn a_class_that_is_not_an_exception_is_discarded() {
    // "net.minecraft.client.Minecraft: startup" is not a crash.
    assert!(split_exception("net.minecraft.client.Minecraft: startup").is_none());
}

/// An exception almost always arrives preceded by the timestamp and category
/// the game's log puts there. What separates them from the class is a "]: "
/// whose length matters: one character too early, the class would carry off
/// the end of the category and no longer be recognized.
#[test]
fn the_timestamp_and_category_are_stripped_before_the_class() {
    let line = "[04:01:47] [main/ERROR]: java.lang.NullPointerException: nothing at this spot";
    let (exception, message) = split_exception(line).expect("the exception follows the category");
    assert_eq!(exception, "java.lang.NullPointerException");
    assert_eq!(message, "nothing at this spot");
}

/// "Caused by: " introduces the real cause, and that's the one wanted: the
/// first exception in a trace is often a wrapper that says nothing.
#[test]
fn the_announced_cause_is_the_one_kept() {
    let (exception, message) =
        split_exception("Caused by: java.io.FileNotFoundException: mods/thing.jar")
            .expect("the cause is an exception");
    assert_eq!(exception, "java.io.FileNotFoundException");
    assert_eq!(message, "mods/thing.jar");
}

/// Three conditions rule out what looks like a class without being one.
/// Each matters on its own: mixing them up would let a log sentence pass
/// for a crash, and the launcher would raise an incident that doesn't exist.
#[test]
fn a_class_without_a_package_with_a_space_or_with_a_slash_is_not_one() {
    // Without a dot: a Java class is named by a dotted path.
    assert!(split_exception("MyException: detail").is_none());
    // With a space: it's a sentence, not an identifier.
    assert!(split_exception("java.lang.An Exception: detail").is_none());
    // With a slash: it's a module or file path.
    assert!(split_exception("java.base/java.lang.Exception: detail").is_none());
}
