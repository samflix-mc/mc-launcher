use super::{GameSession, lines};
use mc_instance::launch::{Command, Session};

/// A session ready to start, as `prepare` builds it.
fn game_session(target: Option<&str>, explicit_request: bool) -> GameSession {
    let layout = mc_instance::Layout::new(std::env::temp_dir().join("mc-pack-announce"));
    GameSession {
        instance: layout.instance("samflix"),
        lock: crate::commands::fixtures::lockfile(Vec::new()),
        version_id: "neoforge-21.1.250".into(),
        command: Command {
            java: std::path::PathBuf::from("/usr/bin/java"),
            args: Vec::new(),
            working_dir: std::env::temp_dir(),
        },
        session: Session::offline("Sam", "b50ad385829d3141a2167e7d7539ba7f"),
        target: target.map(str::to_string),
        explicit_request,
        environment: mc_log::Environment::Production,
    }
}

/// It's the last thing a player reads before the game takes over, and the
/// first thing they paste when asking for help. Each line answers a
/// question asked for real.
#[test]
fn the_summary_names_the_instance_version_player_and_mods() {
    let game_session = game_session(None, false);
    let rendered = lines(&game_session).join("\n");

    assert!(rendered.contains("samflix"), "{rendered}");
    assert!(rendered.contains("neoforge-21.1.250"), "{rendered}");
    assert!(rendered.contains("Sam"), "{rendered}");
    assert!(
        rendered.contains("b50ad385829d3141a2167e7d7539ba7f"),
        "the UUID is missing: {rendered}"
    );
    assert!(rendered.contains("mods"), "{rendered}");
}

/// Where the address comes from matters as much as the address itself:
/// "mc.exemple.fr (production)" made it seem like the host came from the
/// pack, when a `--server` can point anywhere. Someone diagnosing a kick
/// needs to know which of the two they're looking at.
#[test]
fn the_server_says_where_its_address_comes_from() {
    let requested = lines(&game_session(Some("mc.exemple.fr"), true)).join("\n");
    assert!(requested.contains("mc.exemple.fr"), "{requested}");
    assert!(requested.contains("command line"), "{requested}");

    let from_pack = lines(&game_session(Some("mc.exemple.fr"), false)).join("\n");
    assert!(from_pack.contains("declared by the pack"), "{from_pack}");
    // The key shown is the one actually read from the manifest.
    assert!(from_pack.contains("production"), "{from_pack}");
}

/// Preproduction has no server behind it: the game opens on the menu there,
/// and saying so avoids a search for a nonexistent connection problem.
#[test]
fn without_a_server_the_menu_is_announced() {
    let rendered = lines(&game_session(None, false)).join("\n");
    assert!(rendered.contains("none"), "{rendered}");
    assert!(rendered.contains("menu"), "{rendered}");
}
