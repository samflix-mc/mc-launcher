//! Ce que l'observateur reçoit réellement, contre un vrai serveur.
//!
//! Rien ici ne vérifie une mise en forme : c'est le contrat d'émission qui est
//! en jeu. Si les octets cessent d'être annoncés au fil de l'eau, la fenêtre
//! repart pour plusieurs minutes de silence sans qu'aucune compilation ne
//! s'en plaigne.

use std::sync::{Arc, Mutex};

use crate::{Check, Checksum, Downloader, Fetched};

/// Un `Avancement` recopié, parce que celui d'origine emprunte le nom du
/// fichier le temps de l'appel et ne peut pas être conservé.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Trace {
    Lot { fichiers: usize, octets: u64 },
    Debut { fichier: String, octets: Option<u64> },
    Recus(u64),
    Perdus(u64),
    Fini { fichier: String, etat: Fetched, octets: u64 },
}

#[derive(Default)]
struct Journal(Mutex<Vec<Trace>>);

impl Journal {
    fn observateur(self: &Arc<Self>) -> crate::Observateur {
        let journal = Arc::clone(self);
        Arc::new(move |avancement| {
            let trace = match avancement {
                super::Avancement::Lot { fichiers, octets } => Trace::Lot { fichiers, octets },
                super::Avancement::Debut { fichier, octets } => Trace::Debut {
                    fichier: fichier.to_string(),
                    octets,
                },
                super::Avancement::Recus(n) => Trace::Recus(n),
                super::Avancement::Perdus(n) => Trace::Perdus(n),
                super::Avancement::Fini {
                    fichier,
                    etat,
                    octets,
                } => Trace::Fini {
                    fichier: fichier.to_string(),
                    etat,
                    octets,
                },
            };
            journal.0.lock().expect("journal").push(trace);
        })
    }

    fn traces(&self) -> Vec<Trace> {
        self.0.lock().expect("journal").clone()
    }

    /// Ce qui a réellement transité par le réseau, réessais défalqués.
    fn octets_nets(&self) -> i64 {
        self.traces()
            .iter()
            .map(|trace| match trace {
                Trace::Recus(n) => *n as i64,
                Trace::Perdus(n) => -(*n as i64),
                _ => 0,
            })
            .sum()
    }
}

fn dossier(nom: &str) -> std::path::PathBuf {
    let chemin = std::env::temp_dir().join(format!(
        "mc-dl-progression-{nom}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::remove_dir_all(&chemin).ok();
    chemin
}

fn sha1(octets: &[u8]) -> Checksum {
    Checksum::Sha1(Checksum::Sha1(String::new()).of(octets))
}

#[tokio::test]
async fn les_octets_sont_annonces_pendant_la_descente() {
    let serveur = mc_essais::Serveur::neuf().await;
    let corps = vec![b'x'; 64 * 1024];
    serveur.octets("/gros.jar", &corps);

    let journal = Arc::new(Journal::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(journal.observateur());

    dl.bytes(&serveur.url("/gros.jar")).await.unwrap();

    // Le total compte plus que le découpage : le nombre de morceaux dépend de
    // la pile TCP et n'a pas à être figé dans un test.
    assert_eq!(journal.octets_nets(), corps.len() as i64);
    assert!(
        journal
            .traces()
            .iter()
            .any(|t| matches!(t, Trace::Recus(_))),
        "aucun octet annoncé : {:?}",
        journal.traces()
    );
}

/// Un 500 sort avant même que le corps ne commence : rien ne doit avoir été
/// compté, sinon le réessai ferait doubler la barre.
#[tokio::test]
async fn une_reponse_refusee_ne_fait_rien_compter() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.code("/mort", 500);

    let journal = Arc::new(Journal::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(journal.observateur());

    dl.bytes(&serveur.url("/mort")).await.expect_err("500");

    assert_eq!(journal.octets_nets(), 0, "{:?}", journal.traces());
}

/// Trois tentatives dont deux ratées : seul le corps finalement obtenu doit
/// avoir été compté.
#[tokio::test]
async fn un_reessai_ne_compte_pas_deux_fois() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.echoue_puis("/instable", 2, r#"{"ok":true}"#);

    let journal = Arc::new(Journal::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(journal.observateur());

    let recu = dl.bytes(&serveur.url("/instable")).await.unwrap();

    assert_eq!(journal.octets_nets(), recu.len() as i64);
}

#[tokio::test]
async fn un_fichier_telecharge_est_encadre_par_son_nom() {
    let serveur = mc_essais::Serveur::neuf().await;
    let corps = b"contenu du mod";
    serveur.octets("/jei.jar", corps);

    let journal = Arc::new(Journal::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(journal.observateur());
    let dest = dossier("telecharge").join("mods/jei-1.21.1.jar");

    dl.to_file(&serveur.url("/jei.jar"), &dest, Check::Full(&sha1(corps)))
        .await
        .unwrap();

    let traces = journal.traces();
    // Le nom court, pas le chemin : c'est ce qui tient sur une ligne.
    assert_eq!(
        traces.first(),
        Some(&Trace::Debut {
            fichier: "jei-1.21.1.jar".to_string(),
            octets: None,
        }),
        "{traces:?}"
    );
    assert_eq!(
        traces.last(),
        Some(&Trace::Fini {
            fichier: "jei-1.21.1.jar".to_string(),
            etat: Fetched::Downloaded,
            octets: corps.len() as u64,
        }),
        "{traces:?}"
    );
}

/// Le cas d'une réinstallation : tout est déjà là, rien ne descend, et la
/// barre doit malgré tout aller jusqu'au bout.
#[tokio::test]
async fn un_fichier_deja_present_pese_sans_rien_telecharger() {
    let serveur = mc_essais::Serveur::neuf().await;
    let corps = b"deja la";
    serveur.octets("/present.jar", corps);

    let dest = dossier("present").join("present.jar");
    crate::write_atomic(&dest, corps).unwrap();

    let journal = Arc::new(Journal::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(journal.observateur());

    let etat = dl
        .to_file(&serveur.url("/present.jar"), &dest, Check::Full(&sha1(corps)))
        .await
        .unwrap();

    assert_eq!(etat, Fetched::AlreadyPresent);
    assert_eq!(journal.octets_nets(), 0, "rien n'aurait dû transiter");
    assert_eq!(
        journal.traces().last(),
        Some(&Trace::Fini {
            fichier: "present.jar".to_string(),
            etat: Fetched::AlreadyPresent,
            // La taille vient du disque : `Check::Full` n'en publie pas, et
            // zéro ici laisserait la barre immobile.
            octets: corps.len() as u64,
        }),
        "{:?}",
        journal.traces()
    );
}

/// Les lots ne se comptent pas dans `mc-dl` : c'est l'appelant qui sait
/// combien de fichiers il s'apprête à demander, et sans cette annonce aucun
/// temps restant n'est calculable.
#[tokio::test]
async fn un_lot_annonce_par_l_appelant_traverse_tel_quel() {
    let journal = Arc::new(Journal::default());
    let dl = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .observe(journal.observateur());

    dl.signaler(super::Avancement::Lot {
        fichiers: 2_500,
        octets: 830_000_000,
    });

    assert_eq!(
        journal.traces(),
        vec![Trace::Lot {
            fichiers: 2_500,
            octets: 830_000_000
        }]
    );
}

/// Sans observateur — c'est le cas de la ligne de commande — rien ne doit
/// changer de comportement.
#[tokio::test]
async fn sans_observateur_le_telechargement_se_passe_de_la_meme_facon() {
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/liste", r#"{"ok":true}"#);

    let octets = Downloader::new(crate::USER_AGENT)
        .unwrap()
        .bytes(&serveur.url("/liste"))
        .await
        .unwrap();

    assert_eq!(octets, br#"{"ok":true}"#);
}
