//! Chaîne d'authentification d'un launcher Minecraft tiers.
//!
//! Microsoft (device code) → Xbox Live → XSTS → Minecraft → profil.
//!
//! Le *device code flow* évite d'avoir à gérer une URI de redirection et un
//! serveur HTTP local : l'utilisateur ouvre une URL et saisit un code. C'est le
//! flux le plus simple pour une application de bureau, et il est explicitement
//! supporté pour l'API Minecraft.
mod auth;
mod etape;
mod hors_ligne;
mod reponses;

pub use auth::Auth;
pub use etape::{Step, StepError};
pub use hors_ligne::offline_session;
pub use reponses::{DeviceCode, Profile};

/// Compte personnel Microsoft, par opposition à un compte d'organisation.
const TENANT: &str = "consumers";
/// Connexion Xbox Live, et un jeton de rafraîchissement pour ne pas redemander
/// le code à chaque lancement.
const SCOPE: &str = "XboxLive.signin offline_access";

/// Une session ouverte, prête à lancer le jeu.
pub struct Session {
    pub minecraft_token: String,
    pub refresh_token: Option<String>,
    pub profile: Profile,
}
