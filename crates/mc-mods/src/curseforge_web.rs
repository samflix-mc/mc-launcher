//! CurseForge, par l'API publique de son site.
//!
//! La Core API (`api.curseforge.com`) exige une clé nominative : un launcher
//! ne peut pas l'embarquer — elle serait extraite du binaire et révoquée — et
//! l'exiger de chaque joueur revient à lui demander un compte développeur pour
//! installer un modpack. Elle n'est donc plus interrogée du tout.
//!
//! Le site, lui, sert ses propres pages avec une API qui ne demande rien :
//! c'est elle qu'on emprunte ici, complétée par cfwidget pour la seule chose
//! qu'elle refuse, la correspondance entre un slug et un identifiant de projet.
//!
//! C'est donc la seconde et dernière source, après Modrinth. Ce qu'on y perd,
//! et qu'il faut avoir en tête :
//!
//! - **pas d'empreinte.** Seule la taille du fichier est publiée. Le SHA-1 est
//!   donc calculé au premier téléchargement et figé dans le verrou : les
//!   installations suivantes sont vérifiées normalement, seule la toute
//!   première ne l'est pas ;
//! - **pas de `allowModDistribution`.** Seule la Core API expose ce drapeau,
//!   par lequel un auteur refuse d'être téléchargé automatiquement par un
//!   launcher tiers. Il n'est plus lisible. Le téléchargement continue de
//!   passer par la route du site et jamais par une URL de CDN reconstruite —
//!   c'est cette reconstruction qui contournerait activement un refus — mais
//!   le refus lui-même nous échappe désormais ;
//! - **recherche par mot-clé fermée.** Seul un slug exact, tel qu'il apparaît
//!   dans l'adresse de la page du mod, retrouve un projet ;
//! - **cinquante fichiers visibles.** La pagination est ignorée par le serveur
//!   et `pageSize` est plafonné. Un mod qui a publié plus de cinquante fichiers
//!   depuis sa dernière version compatible devient invisible — le cas est
//!   détecté et signalé plutôt que rendu comme « introuvable » ;
//! - **rien de tout cela n'est contractuel.** Ces routes servent le site web,
//!   ne sont pas documentées, et cfwidget est un service tiers bénévole. Les
//!   deux peuvent changer sans préavis, contrairement à Modrinth.
//!
//! Toutes ces routes **enveloppent leur réponse dans `data`**, l'objet isolé
//! comme la liste. Lire un objet sans son enveloppe donne une désérialisation
//! qui échoue, et un build épinglé bien présent déclaré introuvable.

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
