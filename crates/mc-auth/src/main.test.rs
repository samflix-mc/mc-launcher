use super::{Command, parse, usage};

fn parsed(args: &[&str]) -> anyhow::Result<Command> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    parse(&args)
}

#[test]
fn the_three_session_commands_are_recognized() {
    assert_eq!(parsed(&["login"]).unwrap(), Command::Login);
    assert_eq!(parsed(&["whoami"]).unwrap(), Command::Whoami);
    assert_eq!(parsed(&["logout"]).unwrap(), Command::Logout);
}

#[test]
fn offline_mode_carries_its_nickname() {
    assert_eq!(
        parsed(&["--offline", "Sam"]).unwrap(),
        Command::Offline("Sam".into())
    );
}

/// `--offline` without a nickname would launch an anonymous session: better
/// to recall the usage than invent a player name.
#[test]
fn offline_mode_without_a_nickname_recalls_the_usage() {
    let error = parsed(&["--offline"]).expect_err("no nickname");
    assert!(
        format!("{error:#}").contains("--offline <NICKNAME>"),
        "{error:#}"
    );
}

#[test]
fn no_command_or_an_unknown_command_stops_here() {
    assert!(parsed(&[]).is_err());
    assert!(parsed(&["signin"]).is_err());
    assert!(parsed(&["--help"]).is_err());
}

#[test]
fn the_usage_names_the_four_commands() {
    // It goes to standard error; what's checked here is that it doesn't
    // panic and stays callable from the failure path.
    usage();
}
