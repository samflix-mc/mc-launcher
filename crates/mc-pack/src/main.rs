//! Installs a pack, described by a local or published JSON manifest.
//!
//!     mc-pack install                                  the published pack, by default
//!     mc-pack install https://mc-launcher-dev.ggy.info/pack/samflix.json
//!     mc-pack install packs/samflix.json               a manifest from the repo
//!     mc-pack install packs/samflix.json --with-server also installs the server
//!     mc-pack lock    packs/samflix.json               resolves without installing the game
//!     mc-pack verify  [source] [--deep]
//!     mc-pack launch  [source] --username Sam --server mc.exemple.fr
//!     mc-pack diagnostic                               logs and telemetry
//!
//! With no argument, the source is the pack published by mc-content and
//! served by mc-launcher-site: it's the one that decides the mod list, so a
//! player has nothing to clone. A path is still accepted, and that's what you
//! edit with.
//!
//! **Which of the three packs** depends on this binary's environment, which
//! CI fixes at build time: a preproduction launcher downloads the
//! preproduction pack. The address used to be hardcoded in production, so a
//! preproduction build tested none of what it was supposed to test.
//!
//! The pack also designates the server to join, per environment — **when it
//! designates one at all**. Preproduction has no Minecraft servers behind it,
//! so the game opens on the menu there.
//!
//! Common options:
//!     --instance <NAME>  instance name, by default the pack's own
//!     --data <DIR>       root of the launcher's data
//!
//! `RUST_LOG` sets the console's verbosity; the log file keeps the detail no
//! matter what.
mod commands;
mod log;
mod outcome;

use std::process::ExitCode;

use outcome::run;

#[tokio::main]
async fn main() -> ExitCode {
    // Before anything else: an argument-reading error already deserves to be
    // logged, and the guard has to live until the program's very end for the
    // log and the incidents to be sent out completely.
    //
    // It lives until `main` returns, not one instruction less: destructors
    // run before the process hands back its code. That's what the
    // `std::process::exit` calls scattered through the commands used to
    // prevent — a nonconforming `verify` would log its issues, then cut the
    // program before the write queue had put them in the file it had just
    // named itself. Hence the exit code returned, never picked up.
    let _log = mc_log::init("mc-pack");

    match run(&_log).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error:?}");
            ExitCode::FAILURE
        }
    }
}
