//! Comment le jeu s'est terminé, et ce qu'on en dit.

/// Ce qu'une exécution du jeu a produit.
#[derive(Debug)]
pub struct Report {
    pub outcome: Outcome,
    /// Exceptions relevées dans la sortie, que le jeu ait planté ou non.
    ///
    /// Minecraft en rattrape beaucoup et continue : ces erreurs-là
    /// n'apparaissent nulle part ailleurs, et ce sont souvent elles qui
    /// expliquent un comportement signalé bien plus tard.
    pub errors: Vec<crate::crash::Crash>,
}

/// Comment le jeu s'est terminé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Sortie normale, écran de fin ou fermeture de la fenêtre.
    Normal,
    /// Arrêt demandé de l'extérieur : `Ctrl+C`, `kill`, fermeture de session.
    ///
    /// N'est pas une panne. Le confondre avec une erreur remplissait le
    /// tableau de bord d'incidents à chaque fois qu'on fermait le jeu.
    Interrupted { signal: i32 },
    /// Le jeu s'est arrêté de lui-même sur une erreur.
    Failed { code: i32 },
}

impl Outcome {
    pub(super) fn from_status(status: &std::process::ExitStatus) -> Outcome {
        if status.success() {
            return Outcome::Normal;
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Outcome::Interrupted { signal };
            }
            // Un shell traduit un signal en 128 + n. `status.signal()` ne le
            // voit pas quand le code traverse un intermédiaire, d'où cette
            // seconde lecture : 143 est un SIGTERM, 130 un Ctrl+C.
            if let Some(code) = status.code()
                && (129..=192).contains(&code)
            {
                return Outcome::Interrupted { signal: code - 128 };
            }
        }

        Outcome::Failed {
            code: status.code().unwrap_or(-1),
        }
    }

    /// Y a-t-il matière à ouvrir un incident ?
    pub fn is_failure(&self) -> bool {
        matches!(self, Outcome::Failed { .. })
    }
}

#[cfg(test)]
#[path = "compte_rendu.test.rs"]
mod tests;
