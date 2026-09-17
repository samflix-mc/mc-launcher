//! CurseForge sans clé — dernier recours, quand tout le reste a échoué.
//!
//! La Core API (`api.curseforge.com`) exige une clé nominative. Le site web,
//! lui, sert ses propres pages avec une API qui n'en demande pas : c'est elle
//! qu'on emprunte ici, complétée par cfwidget pour la seule chose qu'elle
//! refuse, la correspondance entre un slug et un identifiant de projet.
//!
//! Ce chemin n'est tenté qu'en troisième position — après Modrinth, après la
//! Core API si une clé existe — et il faut savoir ce qu'on y perd :
//!
//! - **pas d'empreinte.** Seule la taille du fichier est publiée. Le SHA-1 est
//!   donc calculé au premier téléchargement et figé dans le verrou : les
//!   installations suivantes sont vérifiées normalement, seule la toute
//!   première ne l'est pas ;
//! - **pas de `allowModDistribution`.** La Core API expose ce drapeau, par
//!   lequel un auteur refuse d'être téléchargé automatiquement par un launcher
//!   tiers. Il est absent de ces routes : ce mode ne peut pas l'honorer, et
//!   c'est la raison pour laquelle il passe en dernier plutôt qu'en premier ;
//! - **cinquante fichiers visibles.** La pagination est ignorée par le serveur
//!   et `pageSize` est plafonné. Un mod qui a publié plus de cinquante fichiers
//!   depuis sa dernière version compatible devient invisible — le cas est
//!   détecté et signalé plutôt que rendu comme « introuvable » ;
//! - **rien de tout cela n'est contractuel.** Ces routes servent le site web,
//!   ne sont pas documentées, et cfwidget est un service tiers bénévole. Les
//!   deux peuvent changer sans préavis, contrairement à Modrinth.

mod api;
mod conversion;
mod epingle;
mod projet;
mod requetes;

use std::sync::Arc;

pub(crate) const WEB: &str = "https://www.curseforge.com/api/v1";
pub(crate) const WIDGET: &str = "https://api.cfwidget.com/minecraft/mc-mods";

/// Plafond imposé par le serveur, quoi qu'on demande.
pub(crate) const PAGE_SIZE: usize = 50;

/// Le client sans clé : mêmes routes que le site lui-même.
pub struct CurseForgeWeb {
    pub(crate) dl: Arc<mc_dl::Downloader>,
    /// Les deux racines sont substituables pour les tests. C'est ici que cela
    /// compte le plus : ces routes ne sont pas documentées et cfwidget est un
    /// service tiers bénévole, donc rien de ce qui suit ne peut être vérifié
    /// contre le vrai service sans le rendre responsable de la CI.
    pub(crate) web: String,
    pub(crate) widget: String,
}

impl CurseForgeWeb {
    pub fn new(dl: Arc<mc_dl::Downloader>) -> Self {
        Self::avec_bases(dl, WEB, WIDGET)
    }

    pub(crate) fn avec_bases(dl: Arc<mc_dl::Downloader>, web: &str, widget: &str) -> Self {
        Self {
            dl,
            web: web.to_string(),
            widget: widget.to_string(),
        }
    }
}
