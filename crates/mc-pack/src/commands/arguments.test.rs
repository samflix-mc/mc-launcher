use super::Arguments;

fn read(args: &[&str]) -> Arguments {
    Arguments::read(args.iter().map(ToString::to_string))
        .expect("valid arguments")
        .expect("a command")
}

/// Without a command, the caller must show help rather than guess.
#[test]
fn without_a_command_there_is_nothing_to_do() {
    let empty: Vec<String> = Vec::new();
    assert!(
        Arguments::read(empty.into_iter())
            .expect("not an error")
            .is_none()
    );
}

/// A bare path is the source: it's the only position that carries no name,
/// and mistaking it for an option would work on the wrong pack.
#[test]
fn the_bare_path_designates_the_pack() {
    let parsed = read(&["lock", "packs/samflix.json"]);
    assert_eq!(parsed.command, "lock");
    assert_eq!(parsed.source_arg.as_deref(), Some("packs/samflix.json"));
}

#[test]
fn value_options_take_the_next_word() {
    let parsed = read(&["launch", "--username", "Sam", "--memory", "4096"]);
    assert_eq!(parsed.username.as_deref(), Some("Sam"));
    assert_eq!(parsed.memory, Some(4096));
}

/// An unknown option stops right there instead of being ignored: a typo on
/// `--username` would otherwise launch the game under another name.
#[test]
fn an_unknown_option_stops_everything() {
    let error = Arguments::read(
        ["launch", "--psuedo", "Sam"]
            .iter()
            .map(ToString::to_string),
    )
    .expect_err("unknown option");
    assert!(error.to_string().contains("--psuedo"), "{error}");
}
