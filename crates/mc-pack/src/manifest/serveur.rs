//! L'adresse d'un serveur, telle qu'un manifeste la déclare.

use serde::{Deserialize, Serialize};

/// Adresse d'un serveur, telle que `--quickPlayMultiplayer` l'attend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    /// Absent = 25565, le port par défaut de Minecraft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

impl Server {
    /// L'adresse telle que `--quickPlayMultiplayer` la lit.
    ///
    /// Minecraft confie cette chaîne à `HostAndPort` de Guava, qui refuse tout
    /// ce qui porte plus d'un « : » sans crochets. Une IPv6 en porte au moins
    /// deux : écrite nue, elle ne donne pas une mauvaise adresse, elle n'en
    /// donne aucune, et le jeu s'ouvre sur le menu sans rien annoncer. D'où les
    /// crochets, posés ici plutôt qu'attendus de la main qui écrit le manifeste.
    ///
    /// Le port est écrit dès qu'il est déclaré, même quand il vaut 25565.
    /// Omettre celui-là paraît sans conséquence, mais Minecraft ne se contente
    /// pas de composer l'adresse : sans port, il interroge l'enregistrement SRV
    /// « _minecraft._tcp.<hôte> » avant de se rabattre sur le défaut. Savoir si
    /// les deux écritures mènent au même serveur demande de connaître la zone
    /// DNS, que le launcher ne voit pas. On ne jette donc pas ce que le
    /// manifeste a pris la peine de dire.
    pub fn address(&self) -> String {
        let host = self.host.trim();
        let hote = if est_ipv6_nue(host) {
            format!("[{host}]")
        } else {
            host.to_string()
        };
        match self.port {
            Some(port) => format!("{hote}:{port}"),
            None => hote,
        }
    }
}

/// Une IPv6 écrite sans ses crochets.
///
/// Deux « : » au moins : une IPv6 en contient toujours au minimum deux, là où
/// « hôte:port » n'en a qu'un. C'est la distinction que fait Guava, donc celle
/// que fait Minecraft.
fn est_ipv6_nue(host: &str) -> bool {
    !host.starts_with('[') && host.matches(':').count() >= 2
}

/// L'hôte porte-t-il déjà un « :port » ?
///
/// Lui en ajouter un second donnerait « hôte:25565:25566 », que Guava refuse
/// comme elle refuse une IPv6 nue.
pub(super) fn porte_deja_un_port(host: &str) -> bool {
    match host.rsplit_once(':') {
        Some((avant, apres)) => {
            (avant.ends_with(']') || !avant.contains(':')) && apres.parse::<u16>().is_ok()
        }
        None => false,
    }
}

#[cfg(test)]
#[path = "serveur.test.rs"]
mod tests;
