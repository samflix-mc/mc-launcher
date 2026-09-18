use super::{Bases, SEGMENT, bases_linux, bases_macos, bases_windows, depuis_bases, du_systeme};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// Un environnement de fausse monnaie, pour éprouver les trois branches sans
/// toucher à celui du processus — que d'autres tests lisent au même moment.
fn environnement(paires: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let table: HashMap<String, OsString> = paires
        .iter()
        .map(|(cle, valeur)| ((*cle).to_string(), OsString::from(*valeur)))
        .collect();
    move |cle: &str| table.get(cle).cloned()
}

fn maison() -> Option<PathBuf> {
    Some(PathBuf::from("/maison/joueur"))
}

fn tmp() -> PathBuf {
    PathBuf::from("/tmp")
}

// --- La déduction ----------------------------------------------------------

/// Les journaux vivent SOUS les données. C'est ce qui fait que supprimer le
/// répertoire de données suffit à repartir de zéro — la seule promesse que
/// cette arborescence fait, et celle qu'un `app_log_dir` du système romprait
/// sous macOS en rangeant ailleurs.
#[test]
fn les_journaux_vivent_sous_les_donnees() {
    let deduit = depuis_bases(Bases {
        donnees: PathBuf::from("/d"),
        config: PathBuf::from("/c"),
        temporaire: PathBuf::from("/t"),
    });

    assert_eq!(deduit.journaux, PathBuf::from("/d/logs"));
    assert!(deduit.journaux.starts_with(&deduit.donnees));
}

/// La déduction ne touche à rien d'autre : ce qu'on lui donne ressort tel
/// quel. C'est ce qui permet à l'application de poser les racines de Tauri et
/// d'obtenir la même arborescence que les crates.
#[test]
fn la_deduction_ne_reecrit_aucune_racine() {
    let bases = Bases {
        donnees: PathBuf::from("/d"),
        config: PathBuf::from("/c"),
        temporaire: PathBuf::from("/t"),
    };
    let deduit = depuis_bases(bases.clone());

    assert_eq!(deduit.donnees, bases.donnees);
    assert_eq!(deduit.config, bases.config);
    assert_eq!(deduit.temporaire, bases.temporaire);
}

// --- Linux -----------------------------------------------------------------

/// `XDG_DATA_HOME` prime quand il est posé — c'est la convention du système,
/// et un poste qui la suit ne doit pas se retrouver avec deux emplacements.
///
/// Ce test remplace `la_convention_du_systeme_est_respectee` de
/// mc-dl/src/emplacements.test.rs, qui écrivait l'environnement du processus
/// pour l'éprouver.
#[test]
fn linux_respecte_la_convention_du_systeme() {
    let bases = bases_linux(
        &environnement(&[
            ("XDG_DATA_HOME", "/ailleurs/partage"),
            ("XDG_CONFIG_HOME", "/ailleurs/reglages"),
        ]),
        maison(),
        tmp(),
    );

    assert_eq!(bases.donnees, PathBuf::from("/ailleurs/partage/samflix-mc"));
    assert_eq!(bases.config, PathBuf::from("/ailleurs/reglages/samflix-mc"));
}

/// Une variable POSÉE MAIS VIDE ne désigne rien. La traiter comme une racine
/// mettrait les données à `/samflix-mc`, à la racine du disque — là où un
/// joueur n'a pas le droit d'écrire, et où personne n'irait chercher.
#[test]
fn linux_ignore_une_variable_vide() {
    let bases = bases_linux(
        &environnement(&[("XDG_DATA_HOME", ""), ("XDG_CONFIG_HOME", "")]),
        maison(),
        tmp(),
    );

    assert_eq!(
        bases.donnees,
        PathBuf::from("/maison/joueur/.local/share/samflix-mc")
    );
    assert_eq!(
        bases.config,
        PathBuf::from("/maison/joueur/.config/samflix-mc")
    );
}

/// Sans variable, les défauts de la spécification XDG.
#[test]
fn linux_retombe_sur_les_defauts_xdg() {
    let bases = bases_linux(&environnement(&[]), maison(), tmp());

    assert_eq!(
        bases.donnees,
        PathBuf::from("/maison/joueur/.local/share/samflix-mc")
    );
    assert_eq!(
        bases.config,
        PathBuf::from("/maison/joueur/.config/samflix-mc")
    );
}

/// Sans `HOME` — un service, un conteneur — on retombe sur le répertoire
/// courant. C'est mauvais, mais visible : une panique serait muette dans une
/// application graphique, qui avale sa sortie d'erreur.
#[test]
fn linux_sans_home_ne_panique_pas() {
    let bases = bases_linux(&environnement(&[]), None, tmp());

    assert_eq!(
        bases.donnees,
        PathBuf::from("./.local/share/samflix-mc"),
        "{:?}",
        bases.donnees
    );
}

/// La séparation qui justifie tout le reste : sous Linux, et là seulement, on
/// peut supprimer huit cents mégaoctets d'instances sans perdre la session.
#[test]
fn linux_separe_les_donnees_de_la_configuration() {
    let bases = bases_linux(&environnement(&[]), maison(), tmp());
    assert_ne!(bases.donnees, bases.config);
    assert!(!bases.config.starts_with(&bases.donnees));
}

// --- macOS -----------------------------------------------------------------

/// `Application Support`, et les deux racines confondues — ce que fait le
/// résolveur de Tauri. S'en écarter ferait diverger l'application de ses
/// propres crates, ce qui est pire qu'un emplacement discutable.
#[test]
fn macos_range_tout_sous_application_support() {
    let bases = bases_macos(&environnement(&[]), maison(), tmp());

    assert_eq!(
        bases.donnees,
        PathBuf::from("/maison/joueur/Library/Application Support/samflix-mc")
    );
    assert_eq!(bases.config, bases.donnees);
}

/// Et la conséquence, écrite ici pour qu'on ne la redécouvre pas : la promesse
/// « supprimer les données sans perdre les préférences » NE VAUT PAS ici.
#[test]
fn macos_ne_separe_pas_les_donnees_de_la_configuration() {
    let bases = bases_macos(&environnement(&[]), maison(), tmp());
    assert_eq!(bases.config, bases.donnees);
}

#[test]
fn macos_sans_home_ne_panique_pas() {
    let bases = bases_macos(&environnement(&[]), None, tmp());
    assert!(bases.donnees.ends_with(SEGMENT), "{:?}", bases.donnees);
}

// --- Windows ---------------------------------------------------------------

#[test]
fn windows_range_tout_sous_appdata() {
    let bases = bases_windows(
        &environnement(&[("APPDATA", r"C:\Users\joueur\AppData\Roaming")]),
        tmp(),
    );

    assert_eq!(
        bases.donnees,
        PathBuf::from(r"C:\Users\joueur\AppData\Roaming").join(SEGMENT)
    );
    assert_eq!(bases.config, bases.donnees);
}

/// Une `APPDATA` vide se traite comme absente : sinon la racine du disque.
#[test]
fn windows_ignore_une_appdata_vide() {
    let bases = bases_windows(&environnement(&[("APPDATA", "")]), tmp());
    assert_eq!(bases.donnees, PathBuf::from(".").join(SEGMENT));
}

#[test]
fn windows_sans_appdata_ne_panique_pas() {
    let bases = bases_windows(&environnement(&[]), tmp());
    assert_eq!(bases.donnees, PathBuf::from(".").join(SEGMENT));
}

// --- Le temporaire, commun aux trois ---------------------------------------

/// Le temporaire porte le segment sur les trois plateformes : sans lui, deux
/// applications qui écrivent un fichier du même nom dans `/tmp` se
/// marcheraient dessus.
#[test]
fn le_temporaire_porte_le_segment_partout() {
    for base in [
        bases_linux(&environnement(&[]), maison(), tmp()),
        bases_macos(&environnement(&[]), maison(), tmp()),
        bases_windows(&environnement(&[]), tmp()),
    ] {
        assert_eq!(base.temporaire, PathBuf::from("/tmp").join(SEGMENT));
    }
}

// --- L'ensemble ------------------------------------------------------------

/// Tout ce que le launcher installe vit sous une seule racine nommée : un seul
/// endroit à supprimer pour repartir de zéro, et rien qui traîne dans le
/// répertoire personnel.
///
/// Reprend `les_donnees_vivent_sous_un_seul_repertoire_nomme` de mc-dl.
#[test]
fn les_donnees_vivent_sous_un_seul_repertoire_nomme() {
    let e = du_systeme();
    assert!(e.donnees.ends_with(SEGMENT), "{:?}", e.donnees);
    assert!(e.donnees.is_absolute() || e.donnees.starts_with("."));
    assert!(e.journaux.starts_with(&e.donnees), "{:?}", e.journaux);
}
