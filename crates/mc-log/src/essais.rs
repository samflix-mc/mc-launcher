//! Ce que les suites de ce crate partagent : l'environnement de déploiement.

/// Sérialise les tests qui posent `SAMFLIX_ENV`.
///
/// La variable est globale au processus, et deux suites de ce crate la
/// regardent : celle de `environment::resolution`, qui la pose pour vérifier
/// d'où vient la valeur, et celle de `incidents::client`, qui vérifie que
/// Sentry annonce l'environnement courant. Sans verrou, la seconde lit ce que
/// la première est en train d'écrire — et `set_var` dans un binaire à plusieurs
/// fils est, depuis l'édition 2024, un comportement indéfini et non une simple
/// course.
///
/// Verrou atomique et non `Mutex` : tenir un `MutexGuard` à travers un `await`
/// est ce que clippy refuse, et ce crate a des suites asynchrones. C'est le
/// même garde que mc-pack tient pour la même raison.
static ENVIRONNEMENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ce qui était déclaré avant le test, et qu'il faut lui rendre.
///
/// La CI lance `cargo test` avec `SAMFLIX_ENV=development` : un garde qui se
/// contenterait d'effacer la variable en se détruisant changerait
/// l'environnement de tous les tests suivants, et ferait dépendre leur
/// résultat de l'ordre d'exécution.
pub(crate) struct Environnement {
    initial: Option<std::ffi::OsString>,
}

fn verrouiller() -> Environnement {
    use std::sync::atomic::Ordering;
    while ENVIRONNEMENT
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    Environnement {
        initial: std::env::var_os("SAMFLIX_ENV"),
    }
}

/// Pose l'environnement de déploiement, et rend le précédent en se détruisant.
pub(crate) fn environnement(valeur: &str) -> Environnement {
    let garde = verrouiller();
    garde.poser(valeur);
    garde
}

/// Prend le verrou sans rien changer : pour les tests qui lisent
/// l'environnement, et qu'une écriture concurrente rendrait faux.
pub(crate) fn environnement_stable() -> Environnement {
    verrouiller()
}

impl Environnement {
    pub(crate) fn poser(&self, valeur: &str) {
        // SAFETY : le verrou garantit qu'aucun autre test de ce binaire ne lit
        // ni n'écrit SAMFLIX_ENV tant que le garde vit.
        unsafe {
            std::env::set_var("SAMFLIX_ENV", valeur);
        }
    }

    /// Retire la déclaration du lancement. Ce qui reste alors est ce que la
    /// compilation a figé — un cas qui se vérifie, et qu'on ne peut pas
    /// atteindre autrement.
    pub(crate) fn retirer(&self) {
        // SAFETY : même verrou, même garantie.
        unsafe {
            std::env::remove_var("SAMFLIX_ENV");
        }
    }
}

impl Drop for Environnement {
    fn drop(&mut self) {
        match &self.initial {
            Some(valeur) => unsafe { std::env::set_var("SAMFLIX_ENV", valeur) },
            None => self.retirer(),
        }
        ENVIRONNEMENT.store(false, std::sync::atomic::Ordering::Release);
    }
}
