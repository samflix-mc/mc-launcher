//! What's shown before handing control back to the game.

use mc_pack::manifest::Manifest;

use mc_pack::GameSession;

/// Out of scope for mutation testing: this function has no effect other
/// than writing to standard output, and Rust has no stable way to read it
/// back from the process that emits it. What it shows, though, is checked
/// line by line — that's [`lines`], right below.
#[mutants::skip]
pub(super) fn announce(game_session: &GameSession) {
    for line in lines(game_session) {
        println!("{line}");
    }
    println!();
}

/// The summary's lines, separate from how they're shown.
///
/// It's the last thing a player reads before the game takes over, and the
/// first thing they paste when asking for help. Each one answers a question
/// asked for real: which instance, which version, under what name, where
/// the mods are, and which server — if there is one.
pub(super) fn lines(game_session: &GameSession) -> Vec<String> {
    let instance = &game_session.instance;
    let session = &game_session.session;

    // Where it comes from is stated, not just the address: "mc.exemple.fr
    // (production)" made it seem like the host came from the pack, when a
    // --server can point anywhere. Someone diagnosing a kick needs to know
    // which of the two they're looking at.
    //
    // The key shown is the one actually read, not the binary's environment:
    // under "local" it's the "development" entry that's used, and showing
    // "local" would send it looking for a key missing from the manifest.
    let key = Manifest::server_environment(game_session.environment);
    let server = match (&game_session.target, game_session.explicit_request) {
        (Some(host), true) => format!("  server  : {host} — requested on the command line"),
        (Some(host), false) => format!(
            "  server  : {host} — declared by the pack for \"{}\"",
            key.as_str()
        ),
        (None, _) => format!(
            "  server  : none for \"{}\" — the game will open on the menu",
            key.as_str()
        ),
    };

    vec![
        format!("Instance \"{}\"", instance.name),
        format!("  version : {}", game_session.version_id),
        format!("  player  : {} ({})", session.name, session.uuid),
        format!("  mods    : {}", instance.mods_dir().display()),
        server,
    ]
}

#[cfg(test)]
#[path = "announce.test.rs"]
mod tests;
