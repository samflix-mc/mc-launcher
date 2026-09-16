//! Où l'on en était quand ça a échoué.

/// Étape de la chaîne, pour situer une erreur sans lire la trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    DeviceCode,
    Token,
    XboxLive,
    Xsts,
    Minecraft,
    Entitlements,
    Profile,
}

#[derive(Debug)]
pub struct StepError {
    pub step: Step,
    pub status: u16,
    pub body: String,
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} a répondu HTTP {} : {}",
            self.step, self.status, self.body
        )
    }
}

impl std::error::Error for StepError {}
