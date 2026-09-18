use super::{IMAGE_MAX, charger, en_data_url, lire, rapatrier, type_mime, url_du_fil};

struct Atelier {
    racine: std::path::PathBuf,
}

impl Atelier {
    fn neuf(nom: &str) -> Atelier {
        let racine = std::env::temp_dir().join(format!(
            "mc-nouvelles-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Atelier { racine }
    }
}

impl Drop for Atelier {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

const FIL: &str = "https://mc-launcher.ggy.info/pack/nouvelles.json";

fn corps(billets: &str) -> String {
    format!(r#"{{"schema":1,"billets":[{billets}]}}"#)
}

fn un_billet(id: &str, date: &str) -> String {
    format!(r#"{{"id":"{id}","titre":"Titre","date":"{date}","corps":"Bonjour."}}"#)
}

/// L'adresse du fil est DÉRIVÉE de celle du pack, jamais recopiée.
///
/// Le même motif que `lock_url_for` : recopier les trois adresses publiées
/// referait le défaut que leur commentaire raconte — une préproduction qui
/// télécharge le contenu de la production, et n'éprouve donc rien de ce
/// qu'elle est censée éprouver.
#[test]
fn l_adresse_du_fil_se_derive_de_celle_du_pack() {
    assert_eq!(
        url_du_fil("https://mc-launcher.ggy.info/pack/samflix.json"),
        "https://mc-launcher.ggy.info/pack/nouvelles.json"
    );
    assert_eq!(
        url_du_fil("https://mc-launcher-dev.ggy.info/pack/samflix.json"),
        "https://mc-launcher-dev.ggy.info/pack/nouvelles.json"
    );
}

/// Un billet fautif est écarté SEUL.
///
/// C'est la différence entre « la page des news a un trou » et « la page des
/// news est vide », et la seconde se lit comme une panne du launcher — alors
/// que la cause est une virgule dans un fichier qu'on ne contrôle pas au même
/// rythme que le code.
#[test]
fn un_billet_fautif_n_emporte_pas_les_autres() {
    let brut = corps(&format!(
        "{},{},{}",
        un_billet("bon-1", "2026-09-01T00:00:00Z"),
        un_billet("date-cassee", "hier"),
        un_billet("bon-2", "2026-09-02T00:00:00Z"),
    ));

    let fil = lire(brut.as_bytes(), FIL).expect("le fil se lit");

    assert_eq!(fil.billets.len(), 2, "{:?}", fil.ecartes);
    assert_eq!(fil.ecartes.len(), 1);
    assert!(fil.ecartes[0].contains("date-cassee"), "{:?}", fil.ecartes);
}

#[test]
fn un_titre_vide_ecarte_le_billet() {
    let brut = corps(r#"{"id":"vide","titre":"  ","date":"2026-09-01T00:00:00Z","corps":"x"}"#);
    let fil = lire(brut.as_bytes(), FIL).unwrap();
    assert!(fil.billets.is_empty());
    assert_eq!(fil.ecartes.len(), 1);
}

/// Une image hors de l'hôte n'écarte pas le billet : elle est simplement
/// ignorée. Perdre tout un billet parce que son illustration est mal rangée
/// serait disproportionné.
#[test]
fn une_image_d_ailleurs_ne_fait_pas_perdre_le_billet() {
    let brut = corps(
        r#"{"id":"x","titre":"T","date":"2026-09-01T00:00:00Z","image":"https://evil.example/i.webp","corps":"x"}"#,
    );
    let fil = lire(brut.as_bytes(), FIL).unwrap();

    assert_eq!(fil.billets.len(), 1);
    assert_eq!(fil.billets[0].image, None);
    assert_eq!(fil.ecartes.len(), 1);
}

#[test]
fn une_image_relative_devient_absolue() {
    let brut = corps(
        r#"{"id":"x","titre":"T","date":"2026-09-01T00:00:00Z","image":"s3.webp","corps":"x"}"#,
    );
    let fil = lire(brut.as_bytes(), FIL).unwrap();
    assert_eq!(
        fil.billets[0].image.as_deref(),
        Some("https://mc-launcher.ggy.info/pack/s3.webp")
    );
}

#[test]
fn un_fil_illisible_rend_une_erreur() {
    assert!(lire(b"{pas du JSON", FIL).is_err());
}

// --- Réseau et repli -------------------------------------------------------

#[tokio::test]
async fn le_fil_se_recupere_et_se_met_en_cache() {
    let atelier = Atelier::neuf("fil-reseau");
    let serveur = mc_essais::Serveur::neuf().await;
    let brut = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    serveur.json("/pack/nouvelles.json", &brut);
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let fil = charger(
        &format!("{}/pack/samflix.json", serveur.base()),
        &atelier.racine,
        &dl,
    )
    .await
    .expect("le fil se charge");

    assert_eq!(fil.billets.len(), 1);
    assert!(!fil.hors_ligne);
    assert!(
        atelier.racine.join("nouvelles.json").is_file(),
        "pas de copie"
    );
}

/// Sans réseau, la copie prend le relais et le dit. Une page de news d'hier
/// vaut mieux qu'une page vide ; une page qui mentirait sur sa fraîcheur
/// serait pire que les deux.
#[tokio::test]
async fn sans_reseau_la_copie_prend_le_relais_et_le_dit() {
    let atelier = Atelier::neuf("fil-hors-ligne");
    let brut = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    std::fs::write(atelier.racine.join("nouvelles.json"), &brut).unwrap();
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let fil = charger("http://127.0.0.1:1/pack/samflix.json", &atelier.racine, &dl)
        .await
        .expect("la copie sert");

    assert_eq!(fil.billets.len(), 1);
    assert!(fil.hors_ligne, "la fenêtre croirait le fil à jour");
}

/// Sans réseau ET sans copie, une erreur franche : c'est la seule situation où
/// l'on n'a rien à montrer, et la page doit le dire plutôt que d'afficher un
/// vide qu'on prendrait pour « aucune nouvelle ».
#[tokio::test]
async fn sans_reseau_ni_copie_l_erreur_est_franche() {
    let atelier = Atelier::neuf("fil-rien");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    assert!(
        charger("http://127.0.0.1:1/pack/samflix.json", &atelier.racine, &dl)
            .await
            .is_err()
    );
}

/// Une réponse illisible ne remplace PAS une copie valide. Sans cette
/// précaution, un hôte qui sert une page d'erreur HTML en HTTP 200 écraserait
/// le dernier fil connu par du vide.
#[tokio::test]
async fn une_reponse_illisible_n_ecrase_pas_la_copie() {
    let atelier = Atelier::neuf("fil-html");
    let bonne = corps(&un_billet("a", "2026-09-01T00:00:00Z"));
    std::fs::write(atelier.racine.join("nouvelles.json"), &bonne).unwrap();

    let serveur = mc_essais::Serveur::neuf().await;
    serveur.json("/pack/nouvelles.json", "<html>maintenance</html>");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let _ = charger(
        &format!("{}/pack/samflix.json", serveur.base()),
        &atelier.racine,
        &dl,
    )
    .await;

    let apres = std::fs::read(atelier.racine.join("nouvelles.json")).unwrap();
    assert_eq!(apres, bonne.as_bytes(), "la copie valide a été écrasée");
}

// --- Les images ------------------------------------------------------------

/// Le type est déduit des OCTETS et non de l'extension : une extension vient
/// de l'URL, donc de l'hôte, donc de quelque chose qu'on ne contrôle pas au
/// même rythme que le code.
#[test]
fn le_type_se_deduit_des_octets() {
    assert_eq!(type_mime(b"\x89PNG\r\n\x1a\nreste"), Some("image/png"));
    assert_eq!(type_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    assert_eq!(type_mime(b"GIF89a....."), Some("image/gif"));
    assert_eq!(type_mime(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));

    assert_eq!(type_mime(b"<html>"), None);
    assert_eq!(type_mime(b""), None);
    // « RIFF » sans « WEBP » est un WAV : une image doit être une image.
    assert_eq!(type_mime(b"RIFF\0\0\0\0WAVEfmt "), None);
}

/// La borne existe parce que `Downloader::bytes` n'en a AUCUNE et accumule le
/// corps entier en mémoire, sous un délai de trois cents secondes. Un hôte
/// fautif servirait un fichier énorme, et le launcher grossirait jusqu'à se
/// faire tuer par le système — sans un message.
#[tokio::test]
async fn une_image_trop_grosse_est_refusee() {
    let atelier = Atelier::neuf("image-grosse");
    let serveur = mc_essais::Serveur::neuf().await;
    let mut enorme = b"\x89PNG\r\n\x1a\n".to_vec();
    enorme.resize(IMAGE_MAX + 1, 0);
    serveur.octets("/grosse.png", &enorme);
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let erreur = rapatrier(&serveur.url("/grosse.png"), &atelier.racine, &dl)
        .await
        .expect_err("une image de plus de 512 kio doit être refusée");
    assert!(format!("{erreur:#}").contains("512"), "{erreur:#}");
}

/// Ce qui n'est pas une image n'est pas mis en cache comme si c'en était une :
/// une page d'erreur HTML servie à la place d'une illustration deviendrait un
/// `data:text/html` chargé dans la fenêtre.
#[tokio::test]
async fn ce_qui_n_est_pas_une_image_est_refuse() {
    let atelier = Atelier::neuf("image-fausse");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/fausse.png", b"<html>404</html>");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    assert!(
        rapatrier(&serveur.url("/fausse.png"), &atelier.racine, &dl)
            .await
            .is_err()
    );
}

/// Le `data:` n'est fabriqué qu'À LA DEMANDE, depuis un fichier.
///
/// Garder les images en base64 dans le cache JSON ferait qu'un fil de huit
/// billets illustrés pèserait trois mégaoctets et demi, relus et resérialisés
/// à chaque ouverture de la page — pour un rendu qui n'en montre qu'une avant
/// qu'on ne fasse défiler.
#[tokio::test]
async fn l_image_est_un_fichier_et_le_data_url_est_fabrique_a_la_demande() {
    let atelier = Atelier::neuf("image-cache");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/i.png", b"\x89PNG\r\n\x1a\ncontenu");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let chemin = rapatrier(&serveur.url("/i.png"), &atelier.racine, &dl)
        .await
        .expect("l'image se rapatrie");

    assert!(chemin.is_file());
    // Un FICHIER, pas du base64 dans un JSON.
    assert_eq!(std::fs::read(&chemin).unwrap(), b"\x89PNG\r\n\x1a\ncontenu");

    let data = en_data_url(&chemin).expect("le data: se fabrique");
    assert!(data.starts_with("data:image/png;base64,"), "{data}");
}

/// Deux billets peuvent porter une `image.webp` chacun : le nom de cache est
/// l'empreinte de l'URL, sans quoi le second écraserait le premier.
#[tokio::test]
async fn deux_images_de_meme_nom_ne_se_marchent_pas_dessus() {
    let atelier = Atelier::neuf("image-collision");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/a/i.png", b"\x89PNG\r\n\x1a\nAAA");
    serveur.octets("/b/i.png", b"\x89PNG\r\n\x1a\nBBB");
    let dl = mc_dl::Downloader::new("essai").unwrap();

    let un = rapatrier(&serveur.url("/a/i.png"), &atelier.racine, &dl)
        .await
        .unwrap();
    let deux = rapatrier(&serveur.url("/b/i.png"), &atelier.racine, &dl)
        .await
        .unwrap();

    assert_ne!(un, deux);
    assert!(std::fs::read(&un).unwrap().ends_with(b"AAA"));
    assert!(std::fs::read(&deux).unwrap().ends_with(b"BBB"));
}

/// Une image déjà en cache n'est pas redemandée. C'est ce qui fait que l'hôte
/// ne compte plus les ouvertures de la page — la seule chose que le
/// rapatriement achète vraiment côté vie privée.
#[tokio::test]
async fn une_image_deja_en_cache_n_est_pas_redemandee() {
    let atelier = Atelier::neuf("image-deja-la");
    let serveur = mc_essais::Serveur::neuf().await;
    serveur.octets("/i.png", b"\x89PNG\r\n\x1a\nX");
    let dl = mc_dl::Downloader::new("essai").unwrap();
    let url = serveur.url("/i.png");

    rapatrier(&url, &atelier.racine, &dl).await.unwrap();
    let apres_un = serveur.recues().len();
    rapatrier(&url, &atelier.racine, &dl).await.unwrap();

    assert_eq!(serveur.recues().len(), apres_un, "l'hôte a été redemandé");
}
