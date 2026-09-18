//! La recette du build empaqueté, jouée par la fenêtre elle-même.
//!
//! ## Ce qu'on vérifiait à l'œil, et pourquoi c'est insuffisant
//!
//! Sept points décident qu'un build empaqueté rend ce que `ng serve` montre,
//! et **aucun ne se voit en développement** : c'est en production seulement
//! que Tauri sert la page, réécrit le CSP et applique les `import()`
//! dynamiques.
//!
//! La recette tenait donc en une liste à dérouler dans l'inspecteur, un point
//! après l'autre. Une liste à dérouler se déroule une fois — celle du jour où
//! on l'a écrite — puis ne se rejoue plus, et ce qu'elle protégeait se casse
//! en silence trois mois plus tard.
//!
//! Ce module la joue à chaque lancement d'une compilation de développement, et
//! la journalise en une ligne. Ce qui manque est nommé ; ce qui va n'est pas
//! bavard.
//!
//! ## Ce qui reste à un humain
//!
//! Ce module dit que `backdrop-filter` **est calculé**, pas qu'il est
//! **joli** ; que les décorations sont absentes, pas que l'ombre de la fenêtre
//! est correcte sous GNOME. Tout ce qui relève du goût ou du gestionnaire de
//! fenêtres reste dans `docs/interface.md` — mais la liste y est désormais
//! courte, et chaque point qui en sort est un point qu'on ne redemandera plus.

use serde::Deserialize;

/// Le nom de l'événement par lequel la fenêtre rapporte.
#[cfg(debug_assertions)]
const EVENEMENT: &str = "recette";

/// Ce que la fenêtre a observé d'elle-même.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Rapport {
    /// La valeur de `--color-primary` sur `:root`, ou vide.
    ///
    /// **La variable, et non l'attribut `data-theme`.** Le thème samflix est
    /// déclaré `default: true` dans le bloc `@plugin` : il n'y a donc aucun
    /// attribut à observer, et en chercher un signalerait un manquement là où
    /// tout va. Ce qui se vérifie est ce qui compte — que les couleurs du
    /// thème soient posées.
    pub theme: String,
    /// La valeur CALCULÉE du `backdrop-filter` de la barre de titre — donc la
    /// preuve qu'un `styleUrl` de composant s'applique, et que le préfixe est
    /// celui que WebKitGTK comprend.
    pub verre: String,
    /// Une couleur passée par `color-mix()`, telle que le moteur la rend.
    pub color_mix: String,
    /// Un composant de ROUTE est-il monté ?
    ///
    /// C'est la preuve qu'un `loadComponent` a résolu son `import()`, donc que
    /// `script-src` laisse passer les fragments paresseux — le seul point de
    /// la recette qui était affirmé sans jamais avoir été démontré.
    pub route_montee: bool,
    /// Combien de feuilles de style le document porte.
    pub feuilles: usize,
    /// Les violations de CSP relevées depuis le tout premier script.
    pub violations: Vec<String>,
}

/// Ce qui manque, nommé.
///
/// Rend une liste vide quand tout va. C'est la moitié PURE du module : elle se
/// relit dans un test, sans serveur d'affichage, et c'est elle qui porte les
/// seuils.
pub fn manquements(rapport: &Rapport) -> Vec<String> {
    let mut manques = Vec::new();

    if rapport.theme.is_empty() {
        manques.push(
            "--color-primary n'est pas posée sur :root : le thème daisyUI n'est pas appliqué"
                .into(),
        );
    }
    // La valeur calculée d'une propriété non reconnue est la chaîne vide ou
    // « none » ; les deux disent la même chose ici — le verre est un aplat.
    if rapport.verre.is_empty() || rapport.verre == "none" {
        manques.push(
            "le backdrop-filter de la barre de titre ne rend rien : \
             soit le style de composant est rejeté, soit le préfixe manque"
                .into(),
        );
    }
    if !rapport.color_mix.starts_with("rgb") && !rapport.color_mix.starts_with("oklab") {
        manques.push(format!(
            "color-mix() n'est pas résolu par le moteur : « {} »",
            rapport.color_mix
        ));
    }
    if rapport.feuilles == 0 {
        manques.push("aucune feuille de style : le CSS n'est pas servi par <link>".into());
    }
    if !rapport.route_montee {
        manques.push(
            "aucun composant de route monté : le fragment paresseux n'est pas arrivé, \
             ce que `script-src` peut casser sans un mot"
                .into(),
        );
    }
    for violation in &rapport.violations {
        manques.push(format!("violation de CSP : {violation}"));
    }

    manques
}

/// Le script injecté AVANT le document.
///
/// Il n'observe rien : il pose seulement le collecteur de violations. C'est la
/// seule façon de les voir toutes — une violation survient au chargement de la
/// page, c'est-à-dire avant tout ce qu'on pourrait exécuter ensuite.
#[cfg(debug_assertions)]
const COLLECTEUR: &str = "window.__violations=[];\
document.addEventListener('securitypolicyviolation',\
e=>window.__violations.push(e.violatedDirective+' ← '+(e.blockedURI||'inline')));";

/// Le greffon qui pose le collecteur.
///
/// Sous forme de greffon parce que c'est la seule manière d'exécuter du script
/// avant le document sur une webview déclarée dans `tauri.conf.json`.
///
/// Il existe dans les deux compilations, et ne pose son script qu'en
/// développement : le `cfg` est À L'INTÉRIEUR plutôt que sur la fonction, ce
/// qui évite au montage de la fenêtre d'avoir deux formes selon le profil —
/// une divergence dont le symptôme serait « cela marche en debug ».
pub fn greffon<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    let constructeur = tauri::plugin::Builder::new("recette");
    #[cfg(debug_assertions)]
    let constructeur = constructeur.js_init_script(COLLECTEUR.to_string());
    constructeur.build()
}

/// Ce que la fenêtre observe d'elle-même, une fois la page affichée.
///
/// **Elle ATTEND qu'un composant de route soit monté**, jusqu'à huit secondes.
/// Sans cette attente, la sonde observe pendant l'amorce — qui tient neuf
/// cents millisecondes, le temps de la poignée de main réseau — et conclut que
/// le fragment paresseux n'est jamais arrivé. C'est ce qu'elle a fait au
/// premier essai, sur un build parfaitement sain.
///
/// Le délai de huit secondes n'est pas un confort : au-delà, ce n'est plus une
/// attente, c'est une panne, et le rapport doit le dire plutôt que de pendre.
#[cfg(debug_assertions)]
const OBSERVATION: &str = "(async()=>{\
const R='app-spawn,app-connexion,app-nouvelles,app-configuration';\
for(let i=0;i<80&&!document.querySelector(R);i++)\
await new Promise(r=>setTimeout(r,100));\
const b=document.querySelector('[data-test=\"barre-titre\"]');\
const s=b?getComputedStyle(b):null;\
const t=document.createElement('div');\
t.style.color='color-mix(in oklab, red 50%, blue)';\
document.body.appendChild(t);\
const cm=getComputedStyle(t).color;\
t.remove();\
const racine=getComputedStyle(document.documentElement);\
return{theme:racine.getPropertyValue('--color-primary').trim(),\
verre:s?(s.backdropFilter||s.webkitBackdropFilter||''):'',\
colorMix:cm,\
routeMontee:!!document.querySelector(R),\
feuilles:document.styleSheets.length,\
violations:window.__violations||[]};})()";

/// Demande la recette à la fenêtre, et journalise ce qui manque.
///
/// Appelée depuis `front_pret`, après le premier rendu : avant, la route
/// affichée n'a pas encore chargé son fragment, et le seul point qui éprouve
/// `import()` rendrait un faux négatif.
#[cfg(debug_assertions)]
pub fn sonder(app: &tauri::AppHandle) {
    use tauri::{Listener, Manager};

    app.once(EVENEMENT, |evenement| {
        let rapport: Rapport = serde_json::from_str(evenement.payload()).unwrap_or_default();
        let manques = manquements(&rapport);
        if manques.is_empty() {
            tracing::info!(
                theme = rapport.theme,
                verre = rapport.verre,
                route_montee = rapport.route_montee,
                feuilles = rapport.feuilles,
                "recette du build : les sept points passent"
            );
            return;
        }
        for manque in manques {
            tracing::error!(manque, "recette du build");
        }
    });

    let Some(fenetre) = app.get_webview_window("main") else {
        return;
    };
    let script = format!(
        "Promise.resolve({OBSERVATION}).then(p=>window.__TAURI_INTERNALS__.invoke(\
         'plugin:event|emit',{{event:'{EVENEMENT}',payload:p}}))"
    );
    if let Err(erreur) = fenetre.eval(&script) {
        tracing::warn!(erreur = %erreur, "recette non exécutée");
    }
}

#[cfg(test)]
#[path = "recette.test.rs"]
mod tests;
