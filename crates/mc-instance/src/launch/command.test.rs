use super::Command;

fn command(args: &[&str]) -> Command {
    Command {
        java: std::path::PathBuf::from("/usr/lib/jvm/temurin-21/bin/java"),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        working_dir: std::path::PathBuf::from("/tmp/instance"),
    }
}

/// The classpath runs to several tens of thousands of characters and no one
/// reads it: displaying it would drown out the line one wanted to reread.
#[test]
fn the_classpath_is_abbreviated_by_its_library_count() {
    let display = command(&["-cp", "/a.jar:/b.jar:/c.jar", "net.minecraft.Main"]).display();

    assert!(!display.contains("/a.jar"), "{display}");
    assert!(display.contains("<3 libraries>"), "{display}");
    // What surrounds it stays readable, and replayable by hand.
    assert!(display.starts_with("/usr/lib/jvm/temurin-21/bin/java "));
    assert!(display.ends_with("net.minecraft.Main"));
}

/// Only the argument that follows `-cp` is abbreviated. Abbreviating the
/// next one too would hide the main class.
#[test]
fn only_the_classpath_argument_is_abbreviated() {
    let display = command(&["-Xmx4096M", "-cp", "/a.jar", "M", "--username", "Sam"]).display();

    assert!(display.contains("-Xmx4096M"), "{display}");
    assert!(display.contains("--username Sam"), "{display}");
    assert!(display.contains("<1 libraries>"), "{display}");
}

#[test]
fn a_command_without_a_classpath_displays_unchanged() {
    let display = command(&["-version"]).display();
    assert!(display.ends_with("java -version"), "{display}");
}
