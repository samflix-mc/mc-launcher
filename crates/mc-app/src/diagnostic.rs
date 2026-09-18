//! Ce que le launcher répond quand on lui demande où il en est, sans ouvrir
//! de fenêtre.
//!
//! `mc-pack diagnostic` existe déjà et sert au dépannage d'un joueur. Celui-ci
//! répond à une autre question, et c'est la CI qui la pose : **le binaire
//! qu'on vient de publier est-il celui qu'on croit ?** Un `.deb` construit
//! sans `SAMFLIX_ENV` compile, s'installe, se lance — et va chercher le pack
//! de développement. Rien ne le signale, puisque le défaut est le silence.
//!
//! D'où un drapeau et non une sous-commande : le binaire est une application
//! graphique, pas un outil en ligne de commande, et il ne doit pas gagner une
//! grammaire d'arguments qu'un joueur pourrait rencontrer par accident.
//!
//! ## Pourquoi la vérification du publié ne porte que sur Linux et macOS
//!
//! `main.rs` pose `windows_subsystem = "windows"` en release : le processus
//! n'a pas de console attachée, et `println!` écrit dans un descripteur qui ne
//! mène nulle part. `AttachConsole(ATTACH_PARENT_PROCESS)` le rattacherait,
//! mais le shell appelant ne l'attend pas et rend la main avant la première
//! ligne — un `grep` de CI y serait instable selon qu'on l'appelle depuis
//! cmd.exe ou PowerShell. Un contrôle qui échoue une fois sur trois se
//! désactive au bout d'un mois, et emporte avec lui les deux fois sur trois
//! où il disait vrai.
//!
//! Le drapeau fonctionne malgré tout sous Windows en compilation de
//! développement, où `windows_subsystem` n'est pas posé.

use std::fmt::Write as _;

/// Le drapeau qui déclenche le diagnostic au lieu de la fenêtre.
const DRAPEAU: &str = "--diagnostic";

/// Le diagnostic est-il demandé ?
///
/// Prend les arguments plutôt que de les lire : c'est ce qui rend la décision
/// vérifiable sans lancer de processus.
pub fn demande<I, S>(arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    arguments.into_iter().any(|a| a.as_ref() == DRAPEAU)
}

/// Ce que le binaire sait de lui-même, en un texte.
///
/// Rend une chaîne au lieu d'imprimer : l'impression appartient à
/// l'appelant, et une fonction qui rend son texte se compare dans un test.
pub fn rapport(dmabuf_desactive: bool) -> String {
    let mut texte = String::new();

    // Le nom d'abord : c'est la seule ligne qui distingue deux binaires
    // construits depuis le même commit avec deux `MC_LAUNCHER_NOM`.
    let _ = writeln!(texte, "Launcher");
    let _ = writeln!(texte, "  nom           : {}", crate::marque::nom());
    let _ = writeln!(
        texte,
        "  version       : {}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("inconnue")
    );

    // La ligne que la CI oppose au tag. `origin()` dit d'où vient la valeur,
    // ce qui distingue « posée à la compilation » de « retombée sur le
    // défaut » — et c'est précisément cette distinction que le contrôle
    // cherche.
    let _ = writeln!(
        texte,
        "  environnement : {} ({})",
        mc_log::environment::current().as_str(),
        mc_log::environment::origin()
    );

    let _ = writeln!(texte, "\nRendu");
    let _ = writeln!(
        texte,
        "  DMA-BUF       : {}",
        if dmabuf_desactive {
            "désactivé (pilote NVIDIA détecté)"
        } else {
            "laissé à WebKit"
        }
    );

    let _ = writeln!(texte, "\nJournaux et incidents");
    let _ = writeln!(texte, "  répertoire    : {}", mc_log::log_dir().display());
    let _ = writeln!(
        texte,
        "  télémétrie    : {}",
        if mc_log::telemetry_active() {
            "active — couper avec SAMFLIX_TELEMETRY=0"
        } else {
            "coupée"
        }
    );

    texte
}

#[cfg(test)]
#[path = "diagnostic.test.rs"]
mod tests;
