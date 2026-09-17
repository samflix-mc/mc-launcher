//! Ce qui se vérifie sans installer huit cents mégaoctets.
//!
//! L'enchaînement lui-même part chez Mojang, Microsoft et CurseForge : ce qui
//! s'en teste ici est la traduction — le rapport qui remplit le suivi, et les
//! quelques champs que le lancement reprend de l'installation.

use super::*;
use mc_pack::Rapport;

fn rapport() -> (Arc<Suivi>, VersLaFenetre) {
    let suivi = Arc::new(Suivi::default());
    (
        Arc::clone(&suivi),
        VersLaFenetre {
            suivi: Arc::clone(&suivi),
        },
    )
}

#[test]
fn une_etape_de_mc_pack_devient_la_phase_correspondante() {
    let (suivi, rapport) = rapport();

    rapport.etape(mc_pack::Etape::Mods);

    assert_eq!(suivi.photo().phase, Phase::Mods);
}

#[test]
fn une_note_est_debarrassee_de_son_indentation_de_terminal() {
    // `mc-pack` met en forme pour une console : « ␣␣128 mods, dont… ». La
    // fenêtre a son propre gabarit, et les espaces de tête y feraient un trou.
    let (suivi, rapport) = rapport();

    rapport.note("  128 mods, dont 43 ajoutés par résolution");

    assert_eq!(
        suivi.photo().note.as_deref(),
        Some("128 mods, dont 43 ajoutés par résolution")
    );
}

#[test]
fn les_telechargements_alimentent_le_meme_compteur() {
    let (suivi, rapport) = rapport();

    rapport.telechargement(mc_dl::Avancement::Lot {
        fichiers: 4,
        octets: 8_000,
    });
    rapport.telechargement(mc_dl::Avancement::Recus(2_000));

    let photo = suivi.photo();
    assert_eq!(photo.octets, 2_000);
    assert_eq!(photo.total, 8_000);
}

#[test]
fn la_cadence_reste_lisible_sans_noyer_le_pont() {
    // Plus vite, on envoie plus de messages que l'écran ne peut en montrer ;
    // plus lentement, un débit qui change devient saccadé. Les deux bornes
    // sont là pour qu'un réglage « pour voir » ne parte pas dans un extrême.
    assert!(CADENCE >= Duration::from_millis(100), "{CADENCE:?}");
    assert!(CADENCE <= Duration::from_millis(500), "{CADENCE:?}");
}

#[test]
fn le_nom_de_l_evenement_ne_bouge_pas() {
    // Le TypeScript écoute cette chaîne-là.
    assert_eq!(EVENEMENT_AVANCEMENT, "cinematique://avancement");
}
