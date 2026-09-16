//! Authentification Minecraft Java : en ligne, ou hors-ligne.
//!
//! ## Pourquoi ce crate ne parle plus à une application Azure
//!
//! Le launcher a longtemps présenté sa propre inscription Azure. Le 16/09/2026,
//! Mojang Enforcement l'a refusée pour la liste blanche de l'API Minecraft,
//! sans motif et sans recours. La chaîne Microsoft → Xbox Live → XSTS
//! fonctionnait pourtant : seul `api.minecraftservices.com` répondait 403,
//! `{"errorMessage":"Invalid app registration"}`, sur la seule foi de
//! l'identifiant d'application.
//!
//! La voie retenue est celle de LiquidBounce, passé par le même refus :
//! s'authentifier avec l'identité du **launcher officiel**
//! (`00000000402b5328`, un *title ID* et non un UUID Azure), via le crate
//! `minecraft-auth`. Le flux n'est pas le même — MSA historique, jeton
//! d'appareil signé ECDSA P-256, SISU en un appel, `/launcher/login` — et c'est
//! la bibliothèque qui le porte.
//!
//! ## Ce que ce choix coûte
//!
//! Ce n'est pas une approbation obtenue, c'est un filtrage contourné. Cela
//! enfreint les conditions d'utilisation de Microsoft et de Mojang. Aucun
//! bannissement lié à cette méthode n'est documenté à ce jour, mais le risque
//! résiduel porterait sur **les joueurs**, pas seulement sur le mainteneur. Le
//! README l'énonce, pour que le choix soit informé.
//!
//! ## Hors-ligne
//!
//! [`offline_session`] reste, et n'a pas changé : elle sert au développement et
//! aux serveurs `online-mode=false`, dont ceux du réseau. Elle ne dépend de
//! rien de tout ce qui précède.

mod auth;
mod hors_ligne;
mod stockage;

pub use auth::{Auth, DeviceCode};
pub use hors_ligne::offline_session;
pub use stockage::{charger, chemin, effacer, enregistrer};

/// Le joueur, tel que le jeu doit l'annoncer.
///
/// `id` est l'UUID en hexadécimal **sans tirets** : c'est la forme que le jeu
/// attend sur sa ligne de commande, et celle que produit déjà
/// [`offline_session`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

/// Une session prête à lancer le jeu.
///
/// Le jeton est vide en mode hors-ligne : c'est la seule différence visible
/// d'ici, et le jeu s'en accommode tant que le serveur tourne en
/// `online-mode=false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub minecraft_token: String,
    pub profile: Profile,
}

impl Session {
    /// La session ouvre-t-elle un serveur en ligne ?
    pub fn est_en_ligne(&self) -> bool {
        !self.minecraft_token.is_empty()
    }
}
