//! Ce qu'il faut faire avant que WebKit ne s'initialise.
//!
//! Depuis WebKitGTK 2.42, le rendu passe par DMA-BUF. Sur le pilote NVIDIA
//! propriétaire, l'échange de tampons échoue : la fenêtre reste blanche, ou se
//! ferme aussitôt ouverte. Le remède connu est
//! `WEBKIT_DISABLE_DMABUF_RENDERER=1`, et c'est une variable d'environnement —
//! donc quelque chose que le développeur tape et que le joueur, lui, ne tapera
//! jamais : il double-clique sur une icône.
//!
//! Elle est donc posée ici, par le programme, pour lui-même.
//!
//! ## Pourquoi pas tout le temps
//!
//! Sans DMA-BUF, WebKit repasse par une copie en mémoire centrale à chaque
//! image. Sur un pilote qui n'a pas le défaut — Intel, AMD, ou NVIDIA en
//! `nouveau` — ce serait payer une régression de rendu pour rien. La variable
//! n'est posée que si le module noyau `nvidia` est chargé.
//!
//! Aucun `cfg` de plateforme : `/sys/module/nvidia` est un chemin Linux, et
//! ailleurs il n'existe pas. Le module est naturellement inerte sous Windows,
//! macOS et Android, sans qu'il faille l'y répéter.
//!
//! ## Pourquoi c'est `unsafe`, et pourquoi c'est sûr ici
//!
//! Depuis l'édition 2024, `set_var` est `unsafe` : écrire l'environnement
//! pendant qu'un autre fil le lit est une course. L'appel est donc la toute
//! première instruction du processus — avant `mc_log::init`, qui ouvre un fil
//! d'écriture pour le journal, et bien avant GTK. C'est la seule fenêtre où
//! l'opération est certaine d'être seule, et c'est pour cela que
//! [`regler_le_rendu`] ne journalise pas : le journal n'existe pas encore. Elle
//! rend ce qu'elle a fait, et l'appelant le dira une fois `mc-log` prêt.

/// Ce que WebKitGTK lit pour savoir s'il doit éviter DMA-BUF.
const VARIABLE: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// Le module noyau, présent seulement avec le pilote propriétaire.
const MODULE_NVIDIA: &str = "/sys/module/nvidia";

/// Pose le contournement si la machine en a besoin. Rend `true` si elle l'a
/// reçu.
///
/// À appeler en premier, avant tout ce qui pourrait créer un fil.
pub fn regler_le_rendu() -> bool {
    if !doit_desactiver_dmabuf(nvidia_charge(), deja_choisi()) {
        return false;
    }

    // SAFETY : premier appel du processus. Aucun fil n'a encore été créé — ni
    // celui du journal, ni ceux de GTK — donc personne ne lit l'environnement
    // pendant qu'on l'écrit.
    unsafe { std::env::set_var(VARIABLE, "1") };
    true
}

/// La décision, isolée de ce qui la met en œuvre.
///
/// `deja_choisi` l'emporte dans les deux sens : qui pose la variable à `0` a
/// une raison de vouloir DMA-BUF malgré NVIDIA — un pilote corrigé, un essai —
/// et l'écraser lui retirerait le seul moyen de le dire.
fn doit_desactiver_dmabuf(nvidia_charge: bool, deja_choisi: bool) -> bool {
    nvidia_charge && !deja_choisi
}

fn nvidia_charge() -> bool {
    std::path::Path::new(MODULE_NVIDIA).exists()
}

fn deja_choisi() -> bool {
    std::env::var_os(VARIABLE).is_some()
}

#[cfg(test)]
#[path = "webkit.test.rs"]
mod tests;
