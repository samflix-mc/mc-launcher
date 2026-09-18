//! The `servers` keys a manifest declares, and the ones that will be read.

use super::server::already_has_a_port;
use super::{Manifest, Server};

impl Manifest {
    /// Meant to run before publishing — not where it's read.
    pub fn server_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (key, server) in &self.servers {
            match mc_log::Environment::parse(key) {
                // A typo causes nothing on read: the key matches no
                // environment, the game opens on the menu, and it looks
                // exactly like a pack that declared nothing. It's the worst
                // kind of silence — the one that looks like intent.
                None => problems.push(format!(
                    "“{key}” isn't an environment: expected development, \
                     preproduction or production"
                )),
                Some(mc_log::Environment::Local) => problems.push(format!(
                    "“{key}” would never be read: a hand-built binary joins \
                     the “development” server"
                )),
                // The canonical name, and only that. Environment::parse
                // accepts common aliases — “dev”, “staging” — but reading
                // looks for “development”: the entry would pass here and
                // stay unreachable there.
                Some(environment) if environment.as_str() != key => problems.push(format!(
                    "“{key}” is an alias; write “{}”, which is the name \
                     looked up on read",
                    environment.as_str()
                )),
                Some(_) => {}
            }
            let host = server.host.trim();
            if host.is_empty() {
                problems.push(format!("the server declared for “{key}” has no host"));
            } else if server.port.is_some() && already_has_a_port(host) {
                problems.push(format!(
                    "the host of “{key}” already carries a port: “{host}” and the \
                     “port” field would give an address with two ports, which \
                     Minecraft rejects"
                ));
            }
        }
        problems
    }

    /// Sends it looking for a key that doesn't exist.
    pub fn server_environment(env: mc_log::Environment) -> mc_log::Environment {
        match env {
            mc_log::Environment::Local => mc_log::Environment::Development,
            other => other,
        }
    }

    /// Server to join for a given environment.
    pub fn server_for(&self, env: mc_log::Environment) -> Option<&Server> {
        self.servers.get(Self::server_environment(env).as_str())
    }
}

#[cfg(test)]
#[path = "servers.test.rs"]
mod tests;

#[cfg(test)]
#[path = "servers.keys.test.rs"]
mod tests_keys;
