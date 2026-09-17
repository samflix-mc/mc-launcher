//! L'enchaînement complet, de la fenêtre ouverte au jeu lancé.
//!
//! Connexion Microsoft, licence, pack, chargeur, fichiers du jeu, Java,
//! NeoForge, mods, verrou — puis le jeu.
//!
//! **Aucune de ces étapes n'est écrite ici.** `mc-auth` authentifie,
//! `mc-pack::install` installe, `mc-pack::jeu` prépare et lance. Ce module
//! appelle, agrège et raconte : c'est tout ce que l'application fait.
//!
//! C'est délibéré. La ligne de commande fait la même chose avec le même code,
//! et deux orchestrations parallèles finiraient par diverger — l'une
//! installerait ce que l'autre ne lancerait pas. Le lancement a d'ailleurs
//! vécu un temps dans le binaire `mc-pack`, hors de portée d'ici ; il est
//! remonté dans la bibliothèque pour cette raison exacte.
//!
//! ## Ce que l'ordre doit à la ligne de commande, et ce qu'il lui ajoute
//!
//! `mc-pack install` n'authentifie rien, et `mc-pack launch` authentifie en
//! premier. Aucune des deux ne vérifie la licence : seul `mc-auth login` le
//! fait, pour information. Une fenêtre ne peut pas se le permettre — demander
//! à un joueur d'attendre huit cents mégaoctets pour lui apprendre ensuite que
//! son compte n'a pas le jeu serait une faute. La licence est donc vérifiée
//! avant d'installer quoi que ce soit, et c'est le seul écart avec le CLI.
//!
//! ## Installer et jouer restent deux gestes
//!
//! `docs/lancement.md` le pose : les enchaîner ferait attendre huit cents
//! mégaoctets à qui voulait seulement jouer. La fenêtre le respecte.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tauri::{AppHandle, Emitter};

use crate::phase::Phase;
use crate::suivi::Suivi;

/// L'événement par lequel l'avancement remonte à la fenêtre.
pub const EVENEMENT_AVANCEMENT: &str = "cinematique://avancement";

/// Entre deux photos envoyées à la fenêtre.
///
/// Cinq par seconde : assez pour qu'un débit paraisse continu, assez peu pour
/// que le pont et le rendu ne coûtent rien. Émettre à chaque morceau reçu
/// enverrait plusieurs milliers de messages par seconde pour un affichage qui
/// ne peut pas en montrer plus de soixante.
const CADENCE: Duration = Duration::from_millis(200);

/// Le rapport que `mc-pack` remplit, et que la fenêtre lit.
///
/// Il ne fait qu'écrire dans le [`Suivi`] : rien n'est émis depuis ici. C'est
/// la boucle de [`emettre`] qui décide quand parler, et elle seule.
struct VersLaFenetre {
    suivi: Arc<Suivi>,
}

impl mc_pack::Rapport for VersLaFenetre {
    fn etape(&self, etape: mc_pack::Etape) {
        self.suivi.phase(Phase::from(etape));
    }

    fn note(&self, texte: &str) {
        // Les notes de `mc-pack` sont mises en forme pour un terminal et
        // commencent parfois par deux espaces d'indentation. La fenêtre les
        // place dans son propre gabarit.
        self.suivi.note(texte.trim());
    }

    fn telechargement(&self, avancement: mc_dl::Avancement<'_>) {
        self.suivi.telechargement(avancement);
    }
}

/// D'où vient le pack, et où il s'installe.
///
/// Les mêmes réglages que la ligne de commande sans argument : l'URL du pack
/// que l'environnement de ce binaire désigne, et la disposition par défaut.
/// Les deux doivent rester identiques, sinon la fenêtre installerait ailleurs
/// que là où `mc-pack launch` va chercher.
fn ou_installer() -> (mc_pack::source::Source, mc_pack::Options) {
    let options = mc_pack::Options::default();
    let source = mc_pack::source::Source::parse(mc_pack::source::url_par_defaut(), &options.layout);
    (source, options)
}

/// Installe le pack, en racontant où l'on en est.
pub async fn installer(app: &AppHandle, suivi: &Arc<Suivi>) -> Result<mc_pack::Outcome> {
    let (source, options) = ou_installer();
    let rapport: Arc<dyn mc_pack::Rapport> = Arc::new(VersLaFenetre {
        suivi: Arc::clone(suivi),
    });

    // La boucle d'émission vit le temps de l'installation et pas au-delà : le
    // `drop` du garde l'arrête, y compris si l'installation échoue.
    let _emission = emettre(app.clone(), Arc::clone(suivi));

    let outcome = mc_pack::install(&source, &options, rapport)
        .await
        .context("installation du pack")?;

    suivi.termine(Phase::Pret);
    pousser(app, suivi);
    Ok(outcome)
}

/// Prépare la partie et lance le jeu.
///
/// Rien n'est réassemblé ici : `mc_pack::jeu::preparer` relit le pack posé sur
/// la machine, reprend le Java du verrou, vérifie que l'instance correspond
/// bien à ce verrou, et construit la ligne de commande. Le refaire à la main
/// aurait sauté la vérification de cohérence — et le serveur aurait tranché
/// par une éjection qui ne nomme pas sa cause.
pub async fn jouer(app: &AppHandle, suivi: &Arc<Suivi>) -> Result<mc_instance::launch::Report> {
    let (source, options) = ou_installer();

    // La préparation lit des descripteurs et crée des répertoires : c'est du
    // disque, et le tenir sur l'exécuteur figerait les autres tâches — dont la
    // boucle qui rafraîchit la fenêtre.
    let partie =
        mc_pack::jeu::preparer(&source, &options, mc_pack::Identite::Microsoft, None, None)
            .await
            .context("préparation de la partie")?;

    suivi.phase(Phase::Lancement);
    suivi.note(&format!("Le jeu démarre — {}", partie.instance.name));
    pousser(app, suivi);

    let rapport = mc_pack::jeu::jouer(&partie).await;

    suivi.termine(Phase::Pret);
    pousser(app, suivi);
    rapport.context("exécution du jeu")
}

/// Émet une photo de l'avancement à cadence fixe, jusqu'à ce qu'on la lâche.
///
/// Le garde rendu arrête la boucle à sa destruction. Sans cela, une
/// installation qui échoue laisserait une tâche parler dans le vide pour le
/// reste de la session.
fn emettre(app: AppHandle, suivi: Arc<Suivi>) -> Emission {
    let (fin, mut arret) = tokio::sync::oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        let mut horloge = tokio::time::interval(CADENCE);
        loop {
            tokio::select! {
                _ = horloge.tick() => pousser(&app, &suivi),
                _ = &mut arret => break,
            }
        }
    });
    Emission { _fin: fin }
}

/// Tant qu'il vit, la fenêtre est rafraîchie.
struct Emission {
    _fin: tokio::sync::oneshot::Sender<()>,
}

/// Une photo, tout de suite.
///
/// Utilisée aux changements qui ne peuvent pas attendre la prochaine cadence —
/// la fin de l'installation, le lancement — pour que l'écran ne reste pas deux
/// dixièmes de seconde sur un état périmé.
fn pousser(app: &AppHandle, suivi: &Arc<Suivi>) {
    if let Err(erreur) = app.emit(EVENEMENT_AVANCEMENT, suivi.photo()) {
        tracing::warn!(erreur = %erreur, "avancement non transmis à la fenêtre");
    }
}

#[cfg(test)]
#[path = "cinematique.test.rs"]
mod tests;
