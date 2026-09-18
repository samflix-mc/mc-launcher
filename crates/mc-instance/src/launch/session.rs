//! Qui joue, et dans quelles conditions.

/// Identité du joueur transmise au jeu.
///
/// Volontairement indépendante de `mc-auth` : une session hors-ligne et une
/// session Microsoft produisent la même structure, et ce module n'a pas à
/// savoir laquelle il sert.
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub uuid: String,
    /// Jeton d'accès. Vide en hors-ligne — le jeu l'accepte et ne rejoint
    /// alors que des serveurs en `online-mode=false`.
    pub token: String,
    /// `msa` pour un compte Microsoft, `legacy` sinon.
    pub user_type: String,
    pub xuid: String,
    pub client_id: String,
}

impl Session {
    /// Session hors-ligne, pour un serveur en `online-mode=false`.
    pub fn offline(name: impl Into<String>, uuid: impl Into<String>) -> Session {
        Session {
            name: name.into(),
            uuid: uuid.into(),
            // Le jeu exige l'argument mais ne le valide pas hors ligne. La
            // valeur « 0 » est celle qu'emploient les launchers usuels : une
            // chaîne vide ferait échouer l'analyse des arguments.
            token: "0".into(),
            user_type: "legacy".into(),
            xuid: String::new(),
            client_id: String::new(),
        }
    }

    /// Session Microsoft, pour un serveur en ligne.
    ///
    /// `xuid` et `client_id` restent vides : ni Prism, ni PolyMC, ni
    /// OpenLauncher ne passent `--xuid` ou `--clientId` au jeu, et le serveur
    /// vérifie l'identité auprès de Mojang à partir du seul jeton.
    pub fn online(
        name: impl Into<String>,
        uuid: impl Into<String>,
        token: impl Into<String>,
    ) -> Session {
        Session {
            name: name.into(),
            uuid: uuid.into(),
            token: token.into(),
            user_type: "msa".into(),
            xuid: String::new(),
            client_id: String::new(),
        }
    }
}

/// Partie à rejoindre directement au démarrage.
#[derive(Debug, Clone)]
pub enum QuickPlay {
    /// Serveur, au format `hôte` ou `hôte:port`.
    Multiplayer(String),
    /// Monde local, par son nom de dossier.
    Singleplayer(String),
}

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    /// Mémoire maximale de la JVM, en mébioctets.
    pub memory_mb: Option<u32>,
    pub quick_play: Option<QuickPlay>,
    pub resolution: Option<(u32, u32)>,
    /// Ouvrir le jeu en plein écran.
    ///
    /// ## Un drapeau, et non une valeur
    ///
    /// `--fullscreen` est un drapeau SANS valeur : écrire `--fullscreen true`
    /// ferait prendre « true » au jeu pour le nom d'un monde à ouvrir. Il est
    /// donc ajouté ou pas ajouté, jamais avec un argument.
    ///
    /// ## Le piège de la double source
    ///
    /// `fullscreen` est AUSSI une clé d'`options.txt`, que F11 bascule en
    /// cours de partie et que le jeu persiste. Piloter le seul argument de
    /// ligne de commande ferait du plein écran un interrupteur à sens unique :
    /// le joueur en sortirait avec F11, et le retrouverait au lancement
    /// suivant sans comprendre pourquoi.
    ///
    /// Les deux sont donc écrits ensemble — voir `mc_reglages::fusionner`.
    pub plein_ecran: bool,
    /// Arguments JVM ajoutés avant ceux du descripteur.
    pub extra_jvm: Vec<String>,
}

#[cfg(test)]
#[path = "session.test.rs"]
mod tests;
