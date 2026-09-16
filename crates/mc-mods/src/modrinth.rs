//! Source Modrinth — consultée en premier, sans clé d'API.
//!
//! Modrinth est prioritaire pour trois raisons concrètes : son API est ouverte
//! (aucune clé à distribuer avec le launcher), elle publie le SHA-1 et le
//! SHA-512 de chaque fichier, et elle expose `client_side` / `server_side` par
//! projet — ce qui donne la répartition client/serveur sans avoir à la saisir
//! à la main dans le manifeste.
//!
//! Elle demande en revanche un `User-Agent` identifiable et applique une limite
//! de débit ; les appels par lots sont donc préférés aux boucles de requêtes.

mod api;
mod conversion;
mod recherche;
mod requetes;

use std::sync::Arc;

pub(crate) const API: &str = "https://api.modrinth.com/v2";

/// Le client Modrinth : source prioritaire, et la seule qui publie un SHA-512.
pub struct Modrinth {
    pub(crate) dl: Arc<mc_dl::Downloader>,
}

impl Modrinth {
    pub fn new(dl: Arc<mc_dl::Downloader>) -> Self {
        Self { dl }
    }
}
