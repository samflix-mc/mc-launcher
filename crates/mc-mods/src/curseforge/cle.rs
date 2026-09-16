//! La clé d'API : où on la trouve, et comment on reconnaît son refus.

/// Clé d'API, cherchée dans l'environnement puis dans la configuration.
///
/// Le fichier permet de ne pas exporter la clé dans chaque shell, et de ne pas
/// la voir passer dans l'historique des commandes.
pub fn api_key() -> Option<String> {
    if let Ok(key) = std::env::var("CURSEFORGE_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Some(key);
        }
    }
    let path = config_key_path();
    let key = std::fs::read_to_string(path).ok()?.trim().to_string();
    (!key.is_empty()).then_some(key)
}

/// Marqueur inséré dans le message quand la clé est en cause.
///
/// Permet au registre de distinguer « cette clé ne vaut rien » — auquel cas il
/// bascule sur l'accès sans clé — d'une panne de réseau, qui doit remonter.
pub(super) const KEY_REFUSED: &str = "CurseForge refuse la clé d'API";

/// L'erreur vient-elle du rejet de la clé ?
pub fn is_key_error(error: &anyhow::Error) -> bool {
    error.chain().any(|e| e.to_string().contains(KEY_REFUSED))
}

pub fn config_key_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("samflix-mc").join("curseforge.key")
}
