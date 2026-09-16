//! Les clés de « servers » qu'un manifeste déclare, et celles qu'on lira.

use super::serveur::porte_deja_un_port;
use super::{Manifest, Server};

impl Manifest {
    /// qu'on lance avant de publier — et non là où il se lit.
    pub fn problemes_de_serveurs(&self) -> Vec<String> {
        let mut problemes = Vec::new();
        for (cle, serveur) in &self.servers {
            match mc_log::Environment::parse(cle) {
                // Une faute de frappe ne provoque rien à la lecture : la clé ne
                // correspond à aucun environnement, le jeu s'ouvre sur le menu,
                // et cela ressemble exactement à un pack qui n'aurait rien
                // déclaré. C'est le pire des silences — celui qui ressemble à
                // une intention.
                None => problemes.push(format!(
                    "« {cle} » n'est pas un environnement : attendus development, \
                     preproduction ou production"
                )),
                Some(mc_log::Environment::Local) => problemes.push(format!(
                    "« {cle} » ne serait jamais lu : un binaire compilé à la main \
                     rejoint le serveur de « development »"
                )),
                // Le nom canonique, et lui seul. Environment::parse accepte les
                // alias courants — « dev », « staging » — mais la lecture
                // cherche « development » : l'entrée passerait ici et resterait
                // introuvable là-bas.
                Some(environnement) if environnement.as_str() != cle => problemes.push(format!(
                    "« {cle} » est un alias ; écrire « {} », qui est le nom cherché \
                     à la lecture",
                    environnement.as_str()
                )),
                Some(_) => {}
            }
            let host = serveur.host.trim();
            if host.is_empty() {
                problemes.push(format!("le serveur déclaré pour « {cle} » n'a pas d'hôte"));
            } else if serveur.port.is_some() && porte_deja_un_port(host) {
                problemes.push(format!(
                    "l'hôte de « {cle} » porte déjà un port : « {host} » et le champ \
                     « port » donneraient une adresse à deux ports, que Minecraft refuse"
                ));
            }
        }
        problemes
    }

    /// l'envoie chercher une clé qui n'existe pas.
    pub fn environnement_serveur(env: mc_log::Environment) -> mc_log::Environment {
        match env {
            mc_log::Environment::Local => mc_log::Environment::Development,
            autre => autre,
        }
    }

    /// Serveur à rejoindre pour un environnement donné.
    pub fn server_for(&self, env: mc_log::Environment) -> Option<&Server> {
        self.servers.get(Self::environnement_serveur(env).as_str())
    }
}

#[cfg(test)]
#[path = "serveurs.test.rs"]
mod tests;

#[cfg(test)]
#[path = "serveurs.cles.test.rs"]
mod tests_cles;
