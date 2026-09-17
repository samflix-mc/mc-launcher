//! Ce qui transforme des milliers d'événements en une ligne lisible.
//!
//! `mc-dl` annonce chaque morceau reçu — soit, sur une installation complète,
//! plusieurs dizaines de milliers d'événements par minute. Les transmettre tels
//! quels à la fenêtre la noierait : le pont Tauri sérialise chacun d'eux, et
//! Angular redessinerait plus souvent que l'écran ne rafraîchit, pour un
//! résultat illisible.
//!
//! Les événements sont donc **accumulés ici**, sans allocation ni verrou sur le
//! chemin chaud, et l'état complet est émis à intervalle fixe. Ce qui arrive à
//! l'écran est une photo prise cinq fois par seconde, pas un flux.
//!
//! ## Ce qui fait avancer la barre
//!
//! Deux sources, qui ne s'additionnent jamais deux fois : les octets descendus
//! du réseau, et le poids des fichiers déjà présents sur le disque. Sans les
//! seconds, une réinstallation resterait à zéro de bout en bout alors que tout
//! est déjà là ; sans les premiers, il n'y aurait aucun débit à afficher.
//!
//! ## Ce qui n'est pas promis
//!
//! Le total vient des lots annoncés par les crates. Il est exact pour Mojang,
//! qui publie les tailles, et c'est un plancher pour les mods de CurseForge
//! sans clé, qui n'en publient pas. Le temps restant s'en déduit : il est une
//! estimation, et l'interface le présente comme tel.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use serde::Serialize;

use crate::phase::Phase;

/// La part du débit mesuré qui entre dans la valeur affichée.
///
/// Le débit instantané d'un lot de petits fichiers saute d'un facteur dix
/// d'une mesure à l'autre — seize requêtes concurrentes qui démarrent et
/// s'achèvent sans se synchroniser. Affiché brut, il clignote et ne se lit pas.
/// Un cinquième de neuf donne une valeur qui met environ une seconde à
/// rejoindre un changement réel, ce qui suffit à suivre une coupure de réseau
/// sans transformer l'affichage en stroboscope.
const LISSAGE: f64 = 0.2;

/// L'état complet, tel qu'il part vers la fenêtre.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Avancement {
    pub phase: Phase,
    /// La dernière note de `mc-pack` — « 128 mods, dont 43 ajoutés… ».
    pub note: Option<String>,
    /// Le fichier en cours de téléchargement.
    pub fichier: Option<String>,
    /// Octets acquis : descendus, plus ceux qui étaient déjà là.
    pub octets: u64,
    /// Ce que le lot en cours a annoncé peser. Zéro quand rien n'est annoncé.
    pub total: u64,
    pub fichiers: usize,
    pub fichiers_total: usize,
    /// Octets par seconde, lissés.
    pub debit: u64,
    /// Secondes restantes, quand le total et le débit permettent de l'estimer.
    pub restant: Option<u64>,
}

/// Le compteur partagé entre les téléchargements et la boucle d'émission.
///
/// Les champs chauds — ceux qu'un morceau reçu touche — sont des atomiques :
/// seize tâches les incrémentent en parallèle, et un verrou sur ce chemin-là
/// se paierait à chaque paquet TCP. Les champs froids, qui changent quelques
/// fois par minute, sont sous `Mutex`.
#[derive(Debug)]
pub struct Suivi {
    phase: Mutex<Phase>,
    note: Mutex<Option<String>>,
    fichier: Mutex<Option<String>>,
    /// Octets réellement descendus, réessais défalqués.
    recus: AtomicU64,
    /// Poids des fichiers trouvés conformes sur le disque.
    deja_la: AtomicU64,
    total: AtomicU64,
    fichiers: AtomicUsize,
    fichiers_total: AtomicUsize,
    /// De quoi calculer un débit : ce qui avait été acquis à la dernière
    /// mesure, et quand.
    mesure: Mutex<Mesure>,
}

#[derive(Debug)]
struct Mesure {
    acquis: u64,
    instant: Instant,
    debit: f64,
}

impl Default for Suivi {
    fn default() -> Self {
        Self {
            phase: Mutex::new(Phase::Connexion),
            note: Mutex::new(None),
            fichier: Mutex::new(None),
            recus: AtomicU64::new(0),
            deja_la: AtomicU64::new(0),
            total: AtomicU64::new(0),
            fichiers: AtomicUsize::new(0),
            fichiers_total: AtomicUsize::new(0),
            mesure: Mutex::new(Mesure {
                acquis: 0,
                instant: Instant::now(),
                debit: 0.0,
            }),
        }
    }
}

impl Suivi {
    pub fn phase(&self, phase: Phase) {
        *self.phase.lock().expect("phase") = phase;
    }

    pub fn note(&self, texte: &str) {
        *self.note.lock().expect("note") = Some(texte.to_string());
    }

    /// Enregistre un événement de téléchargement.
    ///
    /// Appelée depuis les tâches concurrentes de `mc-dl`, des dizaines de
    /// milliers de fois : tout ce qui est fait ici l'est sans verrou, sauf pour
    /// le nom du fichier, qui ne change qu'une fois par fichier.
    pub fn telechargement(&self, avancement: mc_dl::Avancement<'_>) {
        match avancement {
            mc_dl::Avancement::Lot { fichiers, octets } => {
                // Un lot remplace le précédent plutôt qu'il ne s'y ajoute :
                // les étapes s'enchaînent, et la barre repart de zéro à chacune
                // — ce que le chemin des phases explique déjà.
                self.recus.store(0, Ordering::Relaxed);
                self.deja_la.store(0, Ordering::Relaxed);
                self.fichiers.store(0, Ordering::Relaxed);
                self.total.store(octets, Ordering::Relaxed);
                self.fichiers_total.store(fichiers, Ordering::Relaxed);
            }
            mc_dl::Avancement::Debut { fichier, .. } => {
                *self.fichier.lock().expect("fichier") = Some(fichier.to_string());
            }
            mc_dl::Avancement::Recus(octets) => {
                self.recus.fetch_add(octets, Ordering::Relaxed);
            }
            mc_dl::Avancement::Perdus(octets) => {
                // `saturating_sub` sur un compteur atomique s'écrit ainsi :
                // retrancher plus que ce qui est compté ferait repasser par
                // zéro et afficherait seize exaoctets.
                let _ = self
                    .recus
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |acquis| {
                        Some(acquis.saturating_sub(octets))
                    });
            }
            mc_dl::Avancement::Fini { etat, octets, .. } => {
                self.fichiers.fetch_add(1, Ordering::Relaxed);
                if etat == mc_dl::Fetched::AlreadyPresent {
                    self.deja_la.fetch_add(octets, Ordering::Relaxed);
                }
            }
        }
    }

    /// L'état complet, et la mesure du débit depuis le dernier appel.
    ///
    /// Le débit se calcule ici plutôt qu'à chaque morceau parce qu'il n'a de
    /// sens que sur un intervalle : c'est la boucle d'émission qui le fixe, en
    /// appelant à cadence régulière.
    pub fn photo(&self) -> Avancement {
        let acquis = self.recus.load(Ordering::Relaxed) + self.deja_la.load(Ordering::Relaxed);
        let total = self.total.load(Ordering::Relaxed);

        let debit = {
            let mut mesure = self.mesure.lock().expect("mesure");
            let ecoule = mesure.instant.elapsed().as_secs_f64();
            let instantane = debit_instantane(acquis.saturating_sub(mesure.acquis), ecoule);
            mesure.debit = lisser(mesure.debit, instantane);
            mesure.acquis = acquis;
            mesure.instant = Instant::now();
            mesure.debit
        };

        Avancement {
            phase: *self.phase.lock().expect("phase"),
            note: self.note.lock().expect("note").clone(),
            fichier: self.fichier.lock().expect("fichier").clone(),
            octets: acquis,
            total,
            fichiers: self.fichiers.load(Ordering::Relaxed),
            fichiers_total: self.fichiers_total.load(Ordering::Relaxed),
            debit: debit as u64,
            restant: restant(total, acquis, debit),
        }
    }
}

/// Octets par seconde sur l'intervalle qui vient de s'écouler.
///
/// Un intervalle nul — deux appels dans la même microseconde — rendrait
/// l'infini ; on rend zéro, que le lissage absorbe sans faire clignoter
/// l'affichage.
fn debit_instantane(octets: u64, ecoule: f64) -> f64 {
    if ecoule <= 0.0 {
        return 0.0;
    }
    octets as f64 / ecoule
}

/// Moyenne mobile exponentielle.
fn lisser(precedent: f64, mesure: f64) -> f64 {
    precedent * (1.0 - LISSAGE) + mesure * LISSAGE
}

/// Secondes restantes, quand la question a un sens.
///
/// Trois cas rendent `None`, et aucun n'est une erreur : aucun total annoncé
/// (la source ne publie pas les tailles), rien qui descende encore (le débit
/// est nul, diviser donnerait l'infini), et un total déjà dépassé — ce qui
/// arrive quand des mods sans taille publiée comptaient pour zéro. Afficher
/// « 0 s » dans ce dernier cas laisserait croire que c'est fini.
fn restant(total: u64, acquis: u64, debit: f64) -> Option<u64> {
    if total == 0 || debit <= 0.0 || acquis >= total {
        return None;
    }
    Some(((total - acquis) as f64 / debit).ceil() as u64)
}

#[cfg(test)]
#[path = "suivi.test.rs"]
mod tests;
