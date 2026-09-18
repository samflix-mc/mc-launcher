//! Which server to join, and where that choice comes from.

use crate::manifest::Manifest;

/// Absent `--server`, the one the pack declares for this binary's
/// environment.
///
/// The manifest is the same everywhere — it's the same content image, served
/// under three names — so it's up to the client to choose, and it chooses
/// with what CI froze into it at build time.
///
/// An absence is not an error: preproduction has no Minecraft servers behind
/// it, and the game launches there without joining anything.
pub(crate) fn choose(
    manifest: &Manifest,
    server: Option<String>,
) -> (Option<String>, bool, mc_log::Environment) {
    let environment = mc_log::environment::current();
    let explicit_request = server.is_some();
    let target = server.or_else(|| {
        manifest
            .server_for(environment)
            .map(crate::manifest::Server::address)
    });
    (target, explicit_request, environment)
}

#[cfg(test)]
#[path = "target.test.rs"]
mod tests;
