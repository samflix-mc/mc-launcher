//! Ce que les suites de ce crate partagent : les variables d'environnement.
//!
//! Trois d'entre elles décident du comportement observé — `SAMFLIX_ENV` pour
//! l'environnement annoncé, `SAMFLIX_TELEMETRY` et `SENTRY_DSN` pour la
//! remontée d'incidents — et une demi-douzaine de tests les posent. Elles sont
//! globales au processus : deux tests qui les changent en même temps se
//! contredisent, et celui qui ne fait que les lire échoue pour la faute d'un
//! autre. Depuis l'édition 2024, `set_var` dans un binaire à plusieurs fils
//! n'est d'ailleurs plus une course mais un comportement indéfini.
//!
//! D'où ce garde : un verrou unique pour tout le crate, et la restitution de
//! ce qui était déclaré avant. La CI lance la suite avec `SAMFLIX_ENV` posé —
//! un test qui se contenterait d'effacer changerait le résultat de ceux qui le
//! suivent, et ferait dépendre la CI de l'ordre d'exécution.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;

/// Sérialise les tests qui lisent ou posent une variable d'environnement.
///
/// Verrou atomique et non `Mutex` : tenir un `MutexGuard` à travers un `await`
/// est ce que clippy refuse, et ce crate a des suites asynchrones. C'est le
/// même garde que mc-pack tient pour la même raison.
static VERROU: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Variables {
    /// Ce qui était déclaré avant qu'on y touche, par nom. `None` pour une
    /// variable qui n'existait pas — l'effacer est alors la bonne restitution.
    initial: RefCell<HashMap<&'static str, Option<OsString>>>,
}

/// Prend le verrou sans rien changer.
///
/// À tenir aussi pour seulement *lire* : un test qui compare deux lectures de
/// l'environnement serait faux si une écriture se glissait entre elles.
pub(crate) fn variables() -> Variables {
    use std::sync::atomic::Ordering;
    while VERROU
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    Variables {
        initial: RefCell::new(HashMap::new()),
    }
}

impl Variables {
    /// Note ce qui était là, une seule fois par variable : c'est la valeur
    /// d'avant le test qu'il faut rendre, pas celle d'avant le dernier appel.
    fn retenir(&self, nom: &'static str) {
        self.initial
            .borrow_mut()
            .entry(nom)
            .or_insert_with(|| std::env::var_os(nom));
    }

    pub(crate) fn poser(&self, nom: &'static str, valeur: &str) {
        self.retenir(nom);
        // SAFETY : le verrou garantit qu'aucun autre test de ce binaire ne lit
        // ni n'écrit l'environnement tant que le garde vit.
        unsafe {
            std::env::set_var(nom, valeur);
        }
    }

    /// Retire la déclaration du lancement. Pour `SAMFLIX_ENV`, ce qui reste
    /// alors est ce que la compilation a figé — un cas qui se vérifie, et
    /// qu'on ne peut pas atteindre autrement.
    pub(crate) fn retirer(&self, nom: &'static str) {
        self.retenir(nom);
        // SAFETY : même verrou, même garantie.
        unsafe {
            std::env::remove_var(nom);
        }
    }
}

impl Drop for Variables {
    fn drop(&mut self) {
        for (nom, initial) in self.initial.borrow().iter() {
            // SAFETY : le verrou n'est rendu qu'après cette boucle.
            unsafe {
                match initial {
                    Some(valeur) => std::env::set_var(nom, valeur),
                    None => std::env::remove_var(nom),
                }
            }
        }
        VERROU.store(false, std::sync::atomic::Ordering::Release);
    }
}
