use super::{charger, enregistrer};
use crate::types::{Fond, ModeFenetre, Reglages};

/// Un répertoire de travail propre, effacé à la destruction.
struct Atelier {
    racine: std::path::PathBuf,
}

impl Atelier {
    fn neuf(nom: &str) -> Atelier {
        let racine = std::env::temp_dir().join(format!(
            "mc-reglages-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Atelier { racine }
    }

    fn fichier(&self) -> std::path::PathBuf {
        self.racine.join("reglages.json")
    }
}

impl Drop for Atelier {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

/// Sur un poste neuf, les défauts. Pas d'erreur, pas de fichier à créer avant
/// d'ouvrir la fenêtre.
#[test]
fn sans_fichier_les_defauts() {
    let atelier = Atelier::neuf("absent");
    assert_eq!(charger(&atelier.fichier()), Reglages::default());
}

#[test]
fn ce_qui_est_enregistre_se_relit() {
    let atelier = Atelier::neuf("aller-retour");
    let mut voulus = Reglages::default();
    voulus.jeu.render_distance = 20;
    voulus.fenetre.mode = ModeFenetre::PleinEcran;
    voulus.apparence.fond = Fond::Nether;
    voulus.lanceur.memoire_mo = Some(8192);

    enregistrer(&atelier.fichier(), &voulus).expect("écriture");

    assert_eq!(charger(&atelier.fichier()), voulus);
}

/// Un fichier illisible ne doit pas empêcher d'ouvrir le launcher : perdre ses
/// réglages est ennuyeux, ne pas pouvoir jouer l'est davantage.
#[test]
fn un_fichier_casse_donne_les_defauts() {
    let atelier = Atelier::neuf("casse");
    std::fs::write(atelier.fichier(), b"{ceci n'est pas du JSON").unwrap();
    assert_eq!(charger(&atelier.fichier()), Reglages::default());
}

/// Un fichier d'un autre schéma n'est PAS refusé : `#[serde(default)]` sur
/// chaque section fait que ce qui se relit est gardé, et ce qu'on ne connaît
/// pas retombe sur son défaut. Tout perdre serait pire.
#[test]
fn un_autre_schema_garde_ce_qui_se_relit() {
    let atelier = Atelier::neuf("schema");
    std::fs::write(
        atelier.fichier(),
        br#"{"schema":99,"jeu":{"renderDistance":18},"inconnu":{"x":1}}"#,
    )
    .unwrap();

    let relu = charger(&atelier.fichier());
    assert_eq!(
        relu.jeu.render_distance, 18,
        "ce qui se relisait a été perdu"
    );
    // Et le schéma est ramené à celui d'aujourd'hui par `valider`.
    assert_eq!(relu.schema, crate::types::SCHEMA);
}

/// Ce qui est relu est TOUJOURS validé : le fichier s'édite à la main, et
/// c'est exactement ce qu'un joueur qui cherche des images par seconde fera.
/// Une distance de rendu à 200 ne fait pas planter le jeu — elle le fait
/// allouer des gigaoctets jusqu'à l'OutOfMemoryError, vingt minutes plus tard.
#[test]
fn ce_qui_est_relu_est_borne() {
    let atelier = Atelier::neuf("borne-a-la-lecture");
    std::fs::write(
        atelier.fichier(),
        br#"{"schema":1,"jeu":{"renderDistance":200,"maxFps":9999},"apparence":{"voile":0.0}}"#,
    )
    .unwrap();

    let relu = charger(&atelier.fichier());
    assert_eq!(relu.jeu.render_distance, super::super::bornes::RENDU.1);
    assert_eq!(relu.jeu.max_fps, super::super::bornes::FPS.1);
    assert_eq!(relu.apparence.voile, super::super::bornes::VOILE_PLANCHER);
}

/// `enregistrer` rend ce qui a été ÉCRIT et non ce qu'il a reçu.
///
/// Sans cela, une valeur ramenée dans ses bornes laisserait le curseur de la
/// fenêtre à une position que le fichier ne porte pas, jusqu'au prochain
/// rechargement — et le joueur croirait avoir réglé 200 alors que le jeu en
/// recevra 32.
#[test]
fn enregistrer_rend_ce_qui_a_ete_ecrit() {
    let atelier = Atelier::neuf("rend-le-valide");
    let mut aberrants = Reglages::default();
    aberrants.jeu.render_distance = 200;

    let rendus = enregistrer(&atelier.fichier(), &aberrants).expect("écriture");

    assert_eq!(rendus.jeu.render_distance, super::super::bornes::RENDU.1);
    assert_eq!(rendus, charger(&atelier.fichier()));
}

/// Le répertoire parent est créé au besoin : au tout premier enregistrement,
/// le répertoire de configuration peut ne pas exister.
#[test]
fn le_repertoire_est_cree_au_besoin() {
    let atelier = Atelier::neuf("parent");
    let profond = atelier.racine.join("a").join("b").join("reglages.json");
    enregistrer(&profond, &Reglages::default()).expect("écriture");
    assert!(profond.is_file());
}

/// Le fichier de réglages porte un nom, et il est sous la CONFIG.
///
/// Rien ne le vérifiait : `chemin()` pouvait rendre un chemin vide sans qu'un
/// test bronche. Le symptôme serait un launcher qui n'enregistre rien et
/// relit les défauts à chaque ouverture — sans erreur, puisque `charger` ne
/// rend jamais d'erreur.
#[test]
fn les_reglages_ont_un_chemin_sous_la_config() {
    let ou = super::chemin();

    assert!(ou.is_absolute(), "{}", ou.display());
    assert!(ou.ends_with("reglages.json"), "{}", ou.display());
    assert!(
        ou.starts_with(mc_chemins::courants().config),
        "les réglages doivent être sous la config, pas ailleurs : {}",
        ou.display()
    );
}
