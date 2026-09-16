//! Le répertoire personnel, qui porte un nom de compte.

/// Remplace le répertoire personnel par `~`.
///
/// Un chemin absolu porte le nom de compte de l'utilisateur. Ce n'est pas un
/// secret, mais c'est une donnée personnelle qui n'apprend rien de plus que le
/// chemin relatif.
pub(super) fn redact_home(text: &str) -> String {
    match HOME.as_deref() {
        Some(home) if text.contains(home) => text.replace(home, "~"),
        _ => text.to_string(),
    }
}

/// Le répertoire personnel, lu une seule fois.
///
/// `redact` voit passer chaque ligne écrite dans le journal : relire
/// l'environnement à chacune prenait son verrou global — que d'autres fils
/// écrivent par ailleurs — pour une valeur qui ne change pas de l'exécution.
/// La racine « / » est écartée : elle préfixe tout.
static HOME: std::sync::LazyLock<Option<String>> = std::sync::LazyLock::new(|| {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let home = home.to_string_lossy().into_owned();
    (!home.is_empty() && home != "/").then_some(home)
});

#[cfg(test)]
#[path = "home.test.rs"]
mod tests;
