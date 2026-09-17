//! Ce qu'un téléchargement fait savoir pendant qu'il travaille.
//!
//! Le launcher descend près d'un gigaoctet réparti sur quelques milliers de
//! fichiers. Sans rien dire, cela donne une fenêtre figée pendant plusieurs
//! minutes, et un joueur qui ne sait pas distinguer « ça travaille » de « c'est
//! planté ». Le compte rendu final ne sert à rien : il arrive quand l'attente
//! est finie.
//!
//! Ce module ne décide de rien et n'affiche rien. Il ouvre un passage :
//! [`Downloader`](crate::Downloader) émet des [`Avancement`], quelqu'un
//! d'autre les agrège et les montre.
//!
//! ## Pourquoi des deltas et pas un cumul
//!
//! [`Avancement::Recus`] porte ce qui vient d'arriver, jamais un total. Les
//! téléchargements sont concurrents — seize assets en vol à la fois — et un
//! cumul par fichier obligerait l'observateur à tenir un état par fichier pour
//! le recomposer. Des deltas s'additionnent sans rien savoir de qui les envoie.
//!
//! C'est aussi ce qui rend [`Avancement::Perdus`] nécessaire : une tentative
//! qui échoue à mi-corps a déjà fait compter ses octets, et le réessai les
//! recomptera. Sans retrait explicite, une source instable ferait dépasser la
//! barre à 130 %.

use std::sync::Arc;

use crate::Fetched;

/// Un événement de téléchargement.
///
/// Les durées de vie sont empruntées : l'observateur reçoit le nom du fichier
/// pendant l'appel et le copie s'il en a besoin. Copier systématiquement
/// coûterait une allocation par morceau reçu, c'est-à-dire des dizaines de
/// milliers pour rien.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Avancement<'a> {
    /// Un lot commence, et on sait déjà ce qu'il pèse.
    ///
    /// Sans cela, aucun temps restant n'est calculable : le total n'existe
    /// qu'en additionnant des `Content-Length` au fil de l'eau, ce qui donne
    /// un total qui grandit et une barre qui recule. Les trois grands lots —
    /// bibliothèques, assets, mods — connaissent leurs tailles avant de
    /// commencer, et c'est ici qu'ils les annoncent. `octets` vaut zéro quand
    /// la source ne les publie pas.
    Lot { fichiers: usize, octets: u64 },
    /// Un fichier commence à descendre.
    Debut {
        fichier: &'a str,
        octets: Option<u64>,
    },
    /// Des octets viennent d'arriver du réseau. Un delta, jamais un cumul.
    Recus(u64),
    /// Une tentative a échoué à mi-corps : ce qu'elle avait fait compter est à
    /// défalquer avant que le réessai ne le recompte.
    Perdus(u64),
    /// Un fichier est réglé — téléchargé, ou déjà conforme sur le disque.
    ///
    /// `octets` est ce qu'il pèse, pas ce qui a transité : un fichier déjà
    /// présent n'a rien fait descendre et doit pourtant faire avancer la
    /// barre, sans quoi une réinstallation resterait à zéro jusqu'au bout.
    /// C'est ce qui permet à l'observateur de distinguer les deux sans les
    /// additionner deux fois — le réseau se compte par `Recus`, le disque par
    /// ces `Fini`-là.
    Fini {
        fichier: &'a str,
        etat: Fetched,
        octets: u64,
    },
}

/// Qui écoute.
///
/// `Send + Sync` parce que les téléchargements sont concurrents : plusieurs
/// tâches tokio appellent l'observateur en même temps, sans verrou de notre
/// part. À lui de compter de façon atomique.
pub type Observateur = Arc<dyn for<'a> Fn(Avancement<'a>) + Send + Sync>;

#[cfg(test)]
#[path = "progression.test.rs"]
mod tests;
