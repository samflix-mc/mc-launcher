use super::{Watcher, loaded_mods};

/// Feeds lines to a fresh observer, and returns what it drew from them.
fn observe(lines: &[&str]) -> Vec<super::Crash> {
    let mut watcher = Watcher::new();
    for line in lines {
        watcher.line(line);
    }
    watcher.finish()
}

#[test]
fn a_session_without_an_exception_reports_nothing() {
    let found = observe(&[
        "[04:01:48] [main/INFO]: Setting user: Sam",
        "[04:01:49] [Render thread/INFO]: OpenAL initialized.",
    ]);
    assert!(found.is_empty(), "{found:?}");
}

/// A trace continues through its frames; as long as they arrive, they
/// belong to the exception in progress, not to a new one.
#[test]
fn stack_frames_join_the_exception_they_describe() {
    let found = observe(&[
        "java.lang.NullPointerException: nothing at all",
        "\tat net.minecraft.Foo(Foo.java:1)",
        "\tat net.minecraft.Bar(Bar.java:2)",
        "Caused by: java.lang.IllegalStateException: consequence",
        "\t... 12 more",
        "[04:02:00] [main/INFO]: the session continues",
    ]);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].exception, "java.lang.NullPointerException");
    assert!(found[0].excerpt.contains("Foo.java:1"), "{:?}", found[0]);
    assert!(found[0].excerpt.contains("... 12 more"), "{:?}", found[0]);
}

/// A mod that fails on every tick would fill the dashboard on its own: the
/// same exception counts only once.
#[test]
fn the_same_exception_repeated_counts_only_once() {
    let mut lines = Vec::new();
    for _ in 0..10 {
        lines.push("java.lang.NullPointerException: always the same");
        lines.push("\tat net.minecraft.Tick(Tick.java:1)");
    }
    let refs: Vec<&str> = lines.clone();

    let found = observe(&refs);
    assert_eq!(found.len(), 1, "{found:?}");
}

/// Beyond five distinct exceptions, reporting stops: a session that
/// produces this many has a global problem, which the first ones already
/// describe.
#[test]
fn beyond_five_distinct_exceptions_counting_stops() {
    let lines: Vec<String> = (0..12)
        .map(|n| format!("java.lang.IllegalStateException: case number {n}"))
        .collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    let found = observe(&refs);
    assert_eq!(found.len(), 5, "{found:?}");
}

/// The game's output carries ANSI color codes: leaving them in the excerpt
/// would make the incident unreadable in the dashboard.
#[test]
fn terminal_colors_are_stripped() {
    let found = observe(&["\u{1b}[31mjava.io.IOException: disk full\u{1b}[0m"]);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].exception, "java.io.IOException");
    assert!(!found[0].excerpt.contains('\u{1b}'), "{:?}", found[0]);
}

/// The exception in progress when the session stops must not be lost: it's
/// often the one that stopped everything.
#[test]
fn the_exception_in_progress_at_the_end_is_kept() {
    let found = observe(&[
        "java.lang.OutOfMemoryError: Java heap space",
        "\tat net.minecraft.Chunk(Chunk.java:1)",
    ]);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].exception, "java.lang.OutOfMemoryError");
    assert_eq!(
        found[0].source,
        std::path::PathBuf::from("game output"),
        "the origin must say that nothing was read from disk"
    );
}

#[test]
fn two_different_exceptions_are_both_kept() {
    let found = observe(&[
        "java.io.IOException: disk full",
        "[INFO]: on continues",
        "java.lang.NullPointerException: something else",
    ]);

    assert_eq!(found.len(), 2, "{found:?}");
}

/// A modpack crash almost always comes from a mod or a pair of mods; knowing
/// which ones were present saves a round trip.
#[test]
fn loaded_mods_are_listed_in_order() {
    let tree = crate::fixtures::Tree::new("mods-loaded");
    let mods = tree.game_dir().join("mods");
    std::fs::create_dir_all(&mods).unwrap();
    std::fs::write(mods.join("sodium.jar"), b"").unwrap();
    std::fs::write(mods.join("jei.jar"), b"").unwrap();
    // Neither disabled files, nor leftover configuration.
    std::fs::write(mods.join("bug.jar.disabled"), b"").unwrap();

    let names = loaded_mods(&tree.game_dir()).unwrap();
    assert_eq!(names, vec!["jei.jar", "sodium.jar"]);
}

/// A missing mods directory isn't a fault: it's the case for a vanilla
/// instance, and the crash report must still go out.
#[test]
fn a_missing_mods_dir_gives_an_empty_list() {
    let tree = crate::fixtures::Tree::new("mods-missing");
    assert!(loaded_mods(&tree.game_dir()).unwrap().is_empty());
}

/// A trace can run to thousands of frames — an infinite recursion produces
/// them until the stack gives out. The excerpt stops at sixty lines: what
/// follows teaches nothing, and an oversized event gets rejected.
#[test]
fn an_endless_trace_is_bounded() {
    let mut lines = vec!["java.lang.StackOverflowError: stack full".to_string()];
    lines.extend((0..200).map(|i| format!("\tat net.minecraft.Recursion(R.java:{i})")));
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();

    let found = observe(&refs);
    assert_eq!(found.len(), 1, "{found:?}");

    // The declaration, plus sixty frames: not fifty-nine, not sixty-one.
    // It's the bound itself being checked, and it only holds together
    // through a counter that advances by one on each kept line.
    assert_eq!(
        found[0].excerpt.lines().count(),
        1 + super::EXCERPT_LINES,
        "{:?}",
        found[0]
    );
    assert!(found[0].excerpt.contains("R.java:59"), "{:?}", found[0]);
    assert!(!found[0].excerpt.contains("R.java:60"), "{:?}", found[0]);
}
