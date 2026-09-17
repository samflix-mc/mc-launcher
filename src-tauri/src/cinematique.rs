//! L'enchaînement complet, de la fenêtre ouverte au jeu lancé.
//!
//! Connexion Microsoft, licence, pack, chargeur, fichiers du jeu, Java,
//! NeoForge, mods, verrou — puis le bouton « Jouer ».
//!
//! Aucune de ces étapes n'est écrite ici. `mc-auth` authentifie, `mc-pack`
//! installe, `mc-instance` lance ; ce module les appelle dans l'ordre et
//! raconte ce qui se passe. C'est délibéré : la ligne de commande fait la même
//! chose avec le même code, et deux orchestrations parallèles finiraient par
//! diverger — l'une installerait ce que l'autre ne lancerait pas.
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
//! mégaoctets à qui voulait seulement jouer. La fenêtre le respecte — deux
//! boutons, et « Jouer » ne réinstalle pas.

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

/// Installe le pack, en racontant où l'on en est.
///
/// Rend le compte rendu de `mc-pack`, qui porte l'instance, le Java retenu et
/// la version de NeoForge — de quoi lancer ensuite sans rien recalculer.
pub async fn installer(app: &AppHandle, suivi: &Arc<Suivi>) -> Result<mc_pack::Outcome> {
    let options = mc_pack::Options::default();
    let source = mc_pack::source::Source::parse(mc_pack::source::url_par_defaut(), &options.layout);

    let rapport: Arc<dyn mc_pack::Rapport> = Arc::new(VersLaFenetre {
        suivi: Arc::clone(suivi),
    });

    // La boucle d'émission vit le temps de l'installation et pas au-delà : le
    // `drop` du garde l'arrête, y compris si l'installation échoue.
    let _emission = emettre(app.clone(), Arc::clone(suivi));

    let outcome = mc_pack::install(&source, &options, rapport)
        .await
        .context("installation du pack")?;

    suivi.phase(Phase::Pret);
    pousser(app, suivi);
    Ok(outcome)
}

/// Ce qu'une installation réussie laisse derrière elle, et dont le lancement a
/// besoin.
///
/// Trois champs extraits de `mc_pack::Outcome`, qui n'est ni clonable ni
/// transportable à travers un `await`. Les recopier ici évite de tenir un
/// verrou pendant toute une partie de Minecraft.
#[derive(Debug, Clone)]
pub struct Pret {
    /// La version de NeoForge posée, d'où dérive l'identifiant de version.
    pub neoforge: String,
    /// Le Java du verrou — celui avec lequel le chargeur a été installé, pas
    /// celui que le système propose aujourd'hui.
    pub java: std::path::PathBuf,
    pub game_dir: std::path::PathBuf,
}

impl From<&mc_pack::Outcome> for Pret {
    fn from(outcome: &mc_pack::Outcome) -> Self {
        Self {
            neoforge: outcome.neoforge.clone(),
            java: outcome.java.path.clone(),
            game_dir: outcome.instance.game_dir.clone(),
        }
    }
}

/// Prépare la ligne de commande du jeu et l'exécute.
///
/// Reprend ce que l'installation a établi plutôt que de le redécouvrir : le
/// Java est celui du verrou, pas celui du système, et la version de NeoForge
/// celle qui vient d'être posée.
pub async fn jouer(
    app: &AppHandle,
    suivi: &Arc<Suivi>,
    session: mc_instance::launch::Session,
    pret: Pret,
) -> Result<mc_instance::launch::Report> {
    let options = mc_pack::Options::default();
    let version_id = mc_instance::neoforge::version_id(&pret.neoforge);
    let shared = options.layout.shared();
    let java = pret.java;
    let game_dir = pret.game_dir;

    // `build` lit des descripteurs et crée des répertoires : c'est du disque,
    // donc bloquant, et le tenir sur l'exécuteur figerait les autres tâches —
    // dont la boucle qui rafraîchit la fenêtre.
    let commande = tokio::task::spawn_blocking(move || {
        mc_instance::launch::build(
            &version_id,
            &shared,
            &game_dir,
            &java,
            &session,
            &mc_instance::launch::LaunchOptions::default(),
        )
    })
    .await
    .context("préparation de la ligne de commande")??;

    suivi.phase(Phase::Lancement);
    suivi.note("Le jeu est lancé.");
    pousser(app, suivi);

    mc_instance::launch::run(&commande)
        .await
        .context("exécution du jeu")
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
