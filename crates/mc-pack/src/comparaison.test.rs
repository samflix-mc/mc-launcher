use super::{Action, Ecart, comparer, presence, verrou_publie};
use crate::essais::Atelier;
use crate::etat::EtatLocal;
use crate::source::Source;

/// Un verrou publié, servi tel quel.
fn corps_du_verrou(generation: u32) -> String {
    format!(
        r#"{{"schema":1,"name":"samflix","version":"3.2","generated":"2026-09-18T00:00:00Z",
           "minecraft":"1.21.1","loader":{{"type":"neoforge","version":"21.1.250"}},
           "java":21,"generation":{generation},"mods":[]}}"#
    )
}

/// Pose une instance qui a l'air installée, avec l'état qu'on lui donne.
fn poser_instance(atelier: &Atelier, empreinte: &str, generation: u32) {
    let options = atelier.options();
    let instance = options.layout.instance("samflix");
    std::fs::create_dir_all(instance.mods_dir()).unwrap();
    EtatLocal::neuf(empreinte.into(), generation, "2026-09-18T00:00:00Z".into())
        .ecrire(&crate::etat::chemin(&instance))
        .unwrap();
}

// --- presence --------------------------------------------------------------

/// Sur un poste vierge, rien n'est posé. C'est la troisième dette du dépôt :
/// `Etat.installation` n'était peuplé que par une installation faite dans la
/// session courante, si bien qu'au démarrage un pack parfaitement installé
/// paraissait absent.
#[test]
fn sur_un_poste_vierge_rien_n_est_pose() {
    let atelier = Atelier::neuf("presence-vierge");
    let vu = presence(&atelier.options(), "samflix");
    assert!(!vu.installe);
    assert_eq!(vu.etat, None);
}

/// Une instance complète est reconnue SANS qu'une installation ait eu lieu
/// dans cette session-ci. C'est tout l'objet de la fonction.
#[test]
fn une_instance_complete_est_reconnue_au_demarrage() {
    let atelier = Atelier::neuf("presence-complete");
    poser_instance(&atelier, "aa", 0);

    let vu = presence(&atelier.options(), "samflix");
    assert!(vu.installe);
    assert_eq!(vu.etat.unwrap().verrou_sha512, "aa");
}

/// Un état sans répertoire de mods décrit une installation dont quelqu'un a
/// effacé la moitié à la main. Dire « installé » ferait proposer JOUER sur un
/// répertoire vide, et le jeu tomberait au démarrage.
#[test]
fn un_etat_sans_mods_ne_compte_pas_pour_installe() {
    let atelier = Atelier::neuf("presence-moitie");
    let options = atelier.options();
    let instance = options.layout.instance("samflix");
    std::fs::create_dir_all(&instance.dir).unwrap();
    EtatLocal::neuf("aa".into(), 0, "2026-09-18T00:00:00Z".into())
        .ecrire(&crate::etat::chemin(&instance))
        .unwrap();

    assert!(!presence(&options, "samflix").installe);
}

/// Et des mods sans état non plus : c'est une installation d'avant cette
/// version, dont on ne sait rien. Elle repassera par une installation
/// complète, une fois — et c'est cent pour cent de la population.
#[test]
fn des_mods_sans_etat_ne_comptent_pas_pour_installe() {
    let atelier = Atelier::neuf("presence-legs");
    let options = atelier.options();
    std::fs::create_dir_all(options.layout.instance("samflix").mods_dir()).unwrap();

    assert!(!presence(&options, "samflix").installe);
}

// --- verrou_publie ---------------------------------------------------------

/// Le verrou publié se récupère, et c'est bien lui qu'on lit.
#[tokio::test]
async fn le_verrou_publie_se_recupere() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/samflix.lock.json", &corps_du_verrou(0));
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let verrou = verrou_publie(&format!("{}/pack/samflix.json", serveur.base()), &dl)
        .await
        .expect("le verrou publié se lit");

    assert_eq!(verrou.name, "samflix");
    assert_eq!(verrou.java, 21);
}

/// LA frontière que ce module existe pour tenir : une comparaison n'écrit
/// RIEN dans le cache.
///
/// Le seul chemin existant, `load_remote`, sauve manifeste et verrou avant que
/// la moindre installation n'ait commencé. Le réutiliser ferait du cache la
/// description du pack PUBLIÉ et non du pack POSÉ, et casserait la
/// vérification hors-ligne, qui compare le disque à ce cache.
///
/// Ce test est le seul moyen de tenir la frontière dans la durée : elle ne se
/// voit pas dans une signature.
#[tokio::test]
async fn une_comparaison_n_ecrit_rien_dans_le_cache() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/samflix.lock.json", &corps_du_verrou(0));
    let atelier = Atelier::neuf("comparer-sans-ecrire");
    let options = atelier.options();

    let cache = options.layout.cache();
    std::fs::create_dir_all(&cache).unwrap();
    let avant = contenu_recursif(&cache);

    let source = Source::Remote {
        url: format!("{}/pack/samflix.json", serveur.base()),
        cache_dir: cache.clone(),
    };
    let dl = mc_dl::Downloader::new("essai").unwrap();
    comparer(&source, &options, &dl).await;

    assert_eq!(
        contenu_recursif(&cache),
        avant,
        "la comparaison a écrit dans le cache"
    );
}

fn contenu_recursif(racine: &std::path::Path) -> Vec<String> {
    let mut trouves = Vec::new();
    let Ok(entrees) = std::fs::read_dir(racine) else {
        return trouves;
    };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            trouves.extend(contenu_recursif(&chemin));
        } else {
            trouves.push(chemin.display().to_string());
        }
    }
    trouves.sort();
    trouves
}

// --- comparer --------------------------------------------------------------

/// Rien de posé : le bouton dit INSTALLER.
#[tokio::test]
async fn sans_rien_de_pose_le_bouton_dit_installer() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/samflix.lock.json", &corps_du_verrou(0));
    let atelier = Atelier::neuf("comparer-absent");
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", serveur.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.action, Action::Installer);
    assert_eq!(vu.ecart, Ecart::Absent);
    assert!(!vu.installe);
    assert!(!vu.hors_ligne);
    // Le verrou renseigne la page Configuration même quand rien n'est posé.
    assert_eq!(vu.java, Some(21));
    assert_eq!(vu.version.as_deref(), Some("3.2"));
}

/// Posé et identique au publié : à jour, et jouer ne téléchargera rien.
#[tokio::test]
async fn un_pack_identique_au_publie_est_a_jour() {
    let corps = corps_du_verrou(0);
    let empreinte = crate::lockfile::Lockfile::parse(corps.as_bytes())
        .unwrap()
        .empreinte()
        .unwrap();

    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/samflix.lock.json", &corps);
    let atelier = Atelier::neuf("comparer-a-jour");
    poser_instance(&atelier, &empreinte, 0);
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", serveur.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.action, Action::Jouer);
    assert_eq!(vu.ecart, Ecart::AJour);
}

/// Posé mais différent : une mise à jour par différence.
#[tokio::test]
async fn un_verrou_qui_a_bouge_demande_une_mise_a_jour() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/samflix.lock.json", &corps_du_verrou(0));
    let atelier = Atelier::neuf("comparer-maj");
    poser_instance(&atelier, "une-empreinte-d-avant", 0);
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", serveur.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.action, Action::Jouer);
    assert_eq!(vu.ecart, Ecart::MiseAJour);
}

/// La génération l'emporte sur l'empreinte : même si les verrous coïncidaient,
/// une génération supérieure est une demande explicite de réinstallation.
#[tokio::test]
async fn une_generation_superieure_l_emporte_sur_l_empreinte() {
    let corps = corps_du_verrou(2);
    let empreinte = crate::lockfile::Lockfile::parse(corps.as_bytes())
        .unwrap()
        .empreinte()
        .unwrap();

    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/samflix.lock.json", &corps);
    let atelier = Atelier::neuf("comparer-generation");
    // Même empreinte, génération inférieure.
    poser_instance(&atelier, &empreinte, 1);
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::Remote {
            url: format!("{}/pack/samflix.json", serveur.base()),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.ecart, Ecart::Reinstallation);
    assert_eq!(vu.generation, 2);
}

/// Sans réseau, ce n'est PAS une erreur : le bouton dit JOUER si quelque chose
/// est posé. Refuser de jouer parce qu'on n'a pas pu vérifier punirait un
/// joueur d'une panne de réseau.
#[tokio::test]
async fn sans_reseau_un_pack_pose_reste_jouable() {
    let atelier = Atelier::neuf("comparer-hors-ligne");
    poser_instance(&atelier, "aa", 0);
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::Remote {
            // Une adresse qui ne répondra pas.
            url: "http://127.0.0.1:1/pack/samflix.json".into(),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.action, Action::Jouer);
    assert_eq!(vu.ecart, Ecart::Inconnu);
    assert!(vu.hors_ligne);
    assert!(vu.installe);
}

/// Sans réseau et sans rien de posé, le bouton dit INSTALLER — et
/// l'installation échouera franchement faute de pack, ce qui est la bonne
/// façon d'échouer : le joueur apprend qu'il lui faut un réseau.
#[tokio::test]
async fn sans_reseau_et_sans_rien_le_bouton_dit_installer() {
    let atelier = Atelier::neuf("comparer-hors-ligne-vierge");
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::Remote {
            url: "http://127.0.0.1:1/pack/samflix.json".into(),
            cache_dir: options.layout.cache(),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.action, Action::Installer);
    assert!(vu.hors_ligne);
}

/// Un pack LOCAL n'a pas de « publié » : c'est le fichier qu'on édite qui fait
/// foi. On ne prétend pas savoir s'il y a une mise à jour, et le bouton se
/// règle sur la seule présence.
#[tokio::test]
async fn un_pack_local_ne_pretend_pas_connaitre_de_publie() {
    let atelier = Atelier::neuf("comparer-local");
    poser_instance(&atelier, "aa", 0);
    let options = atelier.options();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let vu = comparer(
        &Source::File {
            manifest: atelier.racine.join("samflix.json"),
        },
        &options,
        &dl,
    )
    .await;

    assert_eq!(vu.ecart, Ecart::Inconnu);
    assert_eq!(vu.action, Action::Jouer);
}
