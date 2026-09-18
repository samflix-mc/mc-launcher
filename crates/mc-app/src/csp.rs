//! Ce que le CSP SERVI dit — lu dans la fenêtre, et non dans la configuration.
//!
//! ## Pourquoi la configuration ne suffit pas à répondre
//!
//! `tauri.conf.json` porte une chaîne ; ce que la fenêtre reçoit en est une
//! autre. Entre les deux, Tauri réécrit : il ajoute à `script-src` les
//! EMPREINTES des scripts qu'il injecte lui-même (`manager/mod.rs:64`), et il
//! poserait un nonce sur `style-src` s'il trouvait une balise `<style>` dans
//! le HTML embarqué (`tauri-utils/src/html.rs:154-157`). Or un nonce ANNULE
//! `'unsafe-inline'` — c'est la règle du niveau 3 — et le desserrage sur
//! lequel repose tout style de composant tomberait alors sans un mot, **en
//! build empaqueté seulement**.
//!
//! C'est exactement le genre de divergence qu'on ne découvre pas : la fenêtre
//! ne plante pas, elle s'affiche sans mise en forme.
//!
//! ## Pourquoi une sonde plutôt qu'un coup d'œil dans l'inspecteur
//!
//! L'inspecteur n'existe qu'en compilation de développement, il demande un
//! geste humain, et il ne répond que pour le build qu'on a sous la main ce
//! jour-là. Une sonde qui journalise répond à chaque lancement, et le jour où
//! quelqu'un ajoute un `<style>` dans `index.html` la ligne change toute
//! seule.
//!
//! La moitié qui compte — [`juger`] — est une fonction PURE : elle se relit
//! dans un test, sans serveur d'affichage.

/// Le nom de l'événement par lequel la fenêtre rapporte son en-tête.
#[cfg(debug_assertions)]
const EVENEMENT: &str = "csp-servi";

/// Ce que la lecture de l'en-tête permet de conclure.
#[derive(Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Les styles de composant passent : `style-src` admet l'inline, et rien
    /// ne l'annule.
    pub styles_de_composant_passent: bool,
    /// Un nonce a été posé sur `style-src` — ce qui neutralise
    /// `'unsafe-inline'`, et donc tous les `styleUrl` du front.
    pub nonce_sur_style: bool,
    /// `script-src` n'admet ni `eval` ni inline non couvert par un nonce :
    /// c'est lui qui protège l'origine privilégiée où `invoke` est joignable.
    pub scripts_stricts: bool,
}

/// Juge un en-tête `Content-Security-Policy`.
///
/// Rend `None` si la directive `style-src` est absente : ce n'est pas un
/// verdict par défaut mais une question sans réponse, et la confondre avec
/// « tout va bien » serait le seul moyen de rater ce que la sonde cherche.
pub fn juger(entete: &str) -> Option<Verdict> {
    let style = directive(entete, "style-src")?;
    let script = directive(entete, "script-src").unwrap_or_default();

    // Un nonce ANNULE `'unsafe-inline'` (CSP niveau 3) : les deux présents
    // ensemble se lisent comme le nonce seul, et c'est toute la subtilité que
    // cette sonde existe pour trancher.
    let nonce_sur_style = style.contains("'nonce-");
    let nonce_sur_script = script.contains("'nonce-");

    Some(Verdict {
        styles_de_composant_passent: style.contains("'unsafe-inline'") && !nonce_sur_style,
        nonce_sur_style,
        scripts_stricts: !script.contains("'unsafe-eval'")
            && (!script.contains("'unsafe-inline'") || nonce_sur_script),
    })
}

/// Le contenu d'une directive, ou rien.
///
/// Compare sur le NOM entier et non sur un préfixe : `script-src` est un
/// préfixe de `script-src-elem`, et confondre les deux ferait juger la
/// mauvaise directive — celle qui, justement, n'est pas celle qu'on desserre.
fn directive<'a>(entete: &'a str, nom: &str) -> Option<&'a str> {
    entete.split(';').map(str::trim).find_map(|morceau| {
        let reste = morceau.strip_prefix(nom)?;
        // Le nom doit être suivi d'une espace — ou de rien, pour une
        // directive vide, qui existe et vaut « aucune source ».
        match reste.chars().next() {
            None => Some(""),
            Some(' ') => Some(reste.trim_start()),
            Some(_) => None,
        }
    })
}

/// Demande à la fenêtre l'en-tête qu'elle a réellement reçu, et le journalise.
///
/// Appelée depuis `front_pret`, et de nulle part ailleurs : c'est le seul
/// instant où la page est certainement chargée, donc le seul où un `eval`
/// arrive à destination.
///
/// Absente du binaire de production. Non pas que la ligne y serait dangereuse
/// — elle ne dit rien qu'un `curl` sur le paquet ne dise — mais parce qu'un
/// contrôle de cohérence interne n'a rien à coûter à un joueur.
#[cfg(debug_assertions)]
pub fn sonder(app: &tauri::AppHandle) {
    use tauri::{Listener, Manager};

    app.once(EVENEMENT, |evenement| {
        let entete: String = serde_json::from_str(evenement.payload()).unwrap_or_default();
        match juger(&entete) {
            Some(verdict) => tracing::info!(
                entete,
                styles_de_composant_passent = verdict.styles_de_composant_passent,
                nonce_sur_style = verdict.nonce_sur_style,
                scripts_stricts = verdict.scripts_stricts,
                "CSP servi à la fenêtre"
            ),
            None => tracing::warn!(entete, "CSP servi sans directive style-src"),
        }
    });

    let Some(fenetre) = app.get_webview_window("main") else {
        return;
    };
    // `location.href` et non un chemin écrit en dur : l'origine change selon
    // la plateforme — `tauri://localhost` sous Linux et macOS,
    // `http://tauri.localhost` sous Windows — et un chemin figé ne
    // rapporterait l'en-tête que d'un des trois.
    if let Err(erreur) = fenetre.eval(concat!(
        "fetch(location.href).then(r=>window.__TAURI_INTERNALS__.invoke(",
        "'plugin:event|emit',{event:'csp-servi',",
        "payload:r.headers.get('content-security-policy')||''}))",
    )) {
        tracing::warn!(erreur = %erreur, "sonde CSP non exécutée");
    }
}

#[cfg(test)]
#[path = "csp.test.rs"]
mod tests;
