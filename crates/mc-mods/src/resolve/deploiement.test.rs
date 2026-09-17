use super::deploy;
use crate::essais::Atelier;
use crate::jar::Side;
use crate::resolve::essais::{candidat, installed};
use crate::resolve::plan::{Installed, Plan};

/// Un mod dont le jar existe dans le cache, prêt à être posé.
fn depose(atelier: &Atelier, slug: &str, contenu: &[u8], side: Side) -> Installed {
    let cache = atelier.racine.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let chemin = cache.join(format!("{slug}.jar"));
    std::fs::write(&chemin, contenu).unwrap();

    let mut entree = installed(slug, &[], &[]);
    entree.candidate = candidat(slug, "1.0");
    entree.candidate.sha1 = Some(mc_dl::Checksum::Sha1(String::new()).of(contenu));
    entree.path = chemin;
    entree.side = side;
    entree
}

fn plan(mods: Vec<Installed>) -> Plan {
    Plan {
        mods,
        unresolved: Vec::new(),
    }
}

#[test]
fn les_jars_du_plan_arrivent_dans_le_dossier_mods() {
    let atelier = Atelier::neuf("deploy-simple");
    let mods_dir = atelier.racine.join("mods");
    let plan = plan(vec![
        depose(&atelier, "jei", b"jei", Side::Both),
        depose(&atelier, "sodium", b"sodium", Side::Client),
    ]);

    let pose = deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(pose.installed, 2);
    assert!(pose.removed.is_empty());
    assert!(mods_dir.join("jei.jar").is_file());
    assert!(mods_dir.join("sodium.jar").is_file());
}

/// Un mod client dans le dossier du serveur resterait chargé et ferait diverger
/// le registre du serveur de celui du client.
#[test]
fn un_mod_client_ne_part_pas_sur_le_serveur() {
    let atelier = Atelier::neuf("deploy-cote");
    let mods_dir = atelier.racine.join("mods");
    let plan = plan(vec![
        depose(&atelier, "jei", b"jei", Side::Both),
        depose(&atelier, "sodium", b"sodium", Side::Client),
    ]);

    let pose = deploy(&plan, Side::Server, &mods_dir).unwrap();

    assert_eq!(pose.installed, 1);
    assert!(mods_dir.join("jei.jar").is_file());
    assert!(!mods_dir.join("sodium.jar").exists());
}

/// Ce que le plan ne contient plus doit disparaître : un mod retiré du
/// manifeste mais laissé sur le disque resterait chargé par le jeu.
#[test]
fn un_jar_qui_n_est_plus_au_plan_est_retire() {
    let atelier = Atelier::neuf("deploy-retrait");
    let mods_dir = atelier.racine.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();
    std::fs::write(mods_dir.join("ancien.jar"), b"ancien").unwrap();
    // Ce qui n'est pas un jar ne nous regarde pas : configuration, notes.
    std::fs::write(mods_dir.join("notes.txt"), b"a garder").unwrap();

    let plan = plan(vec![depose(&atelier, "jei", b"jei", Side::Both)]);
    let pose = deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(pose.removed, vec!["ancien.jar"]);
    assert!(!mods_dir.join("ancien.jar").exists());
    assert!(mods_dir.join("notes.txt").is_file());
}

/// Un jar déjà posé et conforme n'est pas recopié : un pack pèse plusieurs
/// centaines de mégaoctets, et le lancement suivant serait inutilement long.
#[test]
fn un_jar_deja_conforme_est_laisse_en_place() {
    let atelier = Atelier::neuf("deploy-idempotent");
    let mods_dir = atelier.racine.join("mods");
    let plan = plan(vec![depose(&atelier, "jei", b"jei", Side::Both)]);

    deploy(&plan, Side::Client, &mods_dir).unwrap();
    let inode = std::fs::metadata(mods_dir.join("jei.jar")).unwrap();
    let pose = deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(pose.installed, 1);
    assert_eq!(
        std::fs::metadata(mods_dir.join("jei.jar")).unwrap().len(),
        inode.len()
    );
}

/// Un jar présent mais altéré — modifié à la main, téléchargement interrompu —
/// doit être remplacé par celui du cache.
#[test]
fn un_jar_altere_est_remplace() {
    let atelier = Atelier::neuf("deploy-altere");
    let mods_dir = atelier.racine.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();
    std::fs::write(mods_dir.join("jei.jar"), b"autre chose").unwrap();

    let plan = plan(vec![depose(&atelier, "jei", b"jei", Side::Both)]);
    deploy(&plan, Side::Client, &mods_dir).unwrap();

    assert_eq!(std::fs::read(mods_dir.join("jei.jar")).unwrap(), b"jei");
}

/// Sans empreinte publiée, un fichier déjà là est accepté tel quel : le
/// remplacer à chaque lancement ne prouverait rien de plus.
#[test]
fn sans_empreinte_un_jar_deja_la_est_accepte() {
    let atelier = Atelier::neuf("deploy-sans-empreinte");
    let mods_dir = atelier.racine.join("mods");
    std::fs::create_dir_all(&mods_dir).unwrap();
    std::fs::write(mods_dir.join("jei.jar"), b"pose a la main").unwrap();

    let mut entree = depose(&atelier, "jei", b"jei", Side::Both);
    entree.candidate.sha1 = None;
    entree.candidate.sha512 = None;

    deploy(&plan(vec![entree]), Side::Client, &mods_dir).unwrap();

    assert_eq!(
        std::fs::read(mods_dir.join("jei.jar")).unwrap(),
        b"pose a la main"
    );
}

#[test]
fn le_dossier_mods_est_cree_au_besoin() {
    let atelier = Atelier::neuf("deploy-creation");
    let mods_dir = atelier
        .racine
        .join("instance")
        .join("minecraft")
        .join("mods");

    let pose = deploy(&plan(Vec::new()), Side::Client, &mods_dir).unwrap();

    assert!(mods_dir.is_dir());
    assert_eq!(pose.installed, 0);
}
