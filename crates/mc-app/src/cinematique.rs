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
//! ## Dans la fenêtre, installer et jouer sont UN SEUL geste
//!
//! `docs/lancement.md` posait le contraire, avec un motif juste : enchaîner les
//! deux ferait attendre huit cents mégaoctets à qui voulait seulement jouer.
//!
//! Ce module RÉSOUT ce motif au lieu de le contredire. Depuis qu'une
//! comparaison d'empreintes coûte quelques dizaines de kilooctets, le cas
//! courant — rien n'a bougé — ne fait plus attendre personne. Et le cas où
//! quelque chose a bougé est précisément celui où ne rien faire donnerait une
//! éjection à la connexion, sans message utile : un joueur qui clique JOUER sur
//! un pack périmé n'a pas choisi de jouer avec un pack périmé, il a choisi de
//! jouer.
//!
//! La ligne de commande, elle, garde ses deux commandes. `install` et `launch`
//! sont des usages d'outilleur, où l'on veut décider soi-même de ce qui se
//! passe — et où l'on n'est pas surpris qu'une commande fasse ce qu'elle dit.

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

/// Ce que le joueur a réglé, tel que `mc-pack` l'attend.
///
/// La traduction se fait ICI et non dans `mc-pack` : la bibliothèque
/// d'installation ne dépend pas de `mc-reglages`, et ne doit pas. Elle reçoit
/// des valeurs, pas une structure de préférences.
///
/// Un fichier de réglages absent ou illisible donne les défauts — jamais une
/// erreur : ne pas pouvoir lancer une partie parce qu'un fichier de confort
/// est corrompu serait absurde.
fn confort_du_joueur() -> mc_pack::jeu::Confort {
    let reglages = mc_reglages::charger(&mc_reglages::chemin());

    mc_pack::jeu::Confort {
        memoire_mo: reglages.lanceur.memoire_mo,
        // `Maximisee` ne passe PAS par une résolution : c'est au jeu de
        // demander la zone utile au gestionnaire de fenêtres, et lui imposer
        // une taille calculée par nous donnerait une fenêtre qui recouvre les
        // panneaux du bureau.
        resolution: match reglages.fenetre.mode {
            mc_reglages::ModeFenetre::Fenetree => {
                Some((reglages.fenetre.largeur, reglages.fenetre.hauteur))
            }
            _ => None,
        },
        plein_ecran: reglages.fenetre.plein_ecran(),
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

/// Ce que le pack a de particulier, sans rien installer.
///
/// Ne touche ni au disque ni au cache : quelques dizaines de kilooctets de
/// réseau pour savoir si le bouton doit dire INSTALLER ou JOUER.
pub async fn etat_du_pack() -> Result<mc_pack::EtatDuPack> {
    let (source, options) = ou_installer();
    let dl = mc_dl::Downloader::new(mc_dl::USER_AGENT).context("client HTTP")?;
    Ok(mc_pack::comparer(&source, &options, &dl).await)
}

/// LE geste : vérifier, rattraper s'il le faut, puis jouer.
///
/// ## La garde d'émission est en TÊTE, et c'est le point
///
/// Elle vivait dans `installer`, et `jouer` n'en posait aucune. Or c'est
/// pendant la COMPARAISON que la fenêtre a l'air figée : quelques centaines de
/// millisecondes de réseau pendant lesquelles aucune étape ne s'allume, avant
/// même qu'on sache s'il y aura une installation. La poser après la
/// comparaison laisserait exactement ce trou.
pub async fn mettre_a_jour_et_jouer(
    app: &AppHandle,
    suivi: &Arc<Suivi>,
) -> Result<mc_pack::Deroulement> {
    let (source, options) = ou_installer();

    // AVANT tout le reste.
    let _emission = emettre(app.clone(), Arc::clone(suivi));

    let rapport: Arc<dyn mc_pack::Rapport> = Arc::new(VersLaFenetre {
        suivi: Arc::clone(suivi),
    });

    let deroulement = mc_pack::mettre_a_jour_et_jouer(
        &source,
        &options,
        mc_pack::Identite::Microsoft,
        None,
        confort_du_joueur(),
        rapport,
    )
    .await
    .context("lancement de la partie")?;

    suivi.termine(Phase::Pret);
    pousser(app, suivi);
    Ok(deroulement)
}

/// Vérifie les fichiers de l'instance posée, sans rien télécharger.
///
/// Le geste de la section « Avancé ». Il a quitté l'écran principal avec la
/// refonte, et c'est ici qu'il réapparaît — sans quoi il n'aurait plus de porte
/// d'entrée graphique du tout.
///
/// `profond` décide de ce qu'on compare : le nom et la taille, ou l'empreinte
/// de chaque fichier. La seconde relit plusieurs centaines de mégaoctets, et
/// c'est pour cela que la page le demande explicitement.
pub fn verifier(profond: bool) -> Result<Vec<String>> {
    let (source, options) = ou_installer();
    mc_pack::verify(&source, &options, profond).context("vérification de l'instance")
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
