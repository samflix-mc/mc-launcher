use super::*;

fn versions(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn compatibilite_lue_dans_game_versions() {
    // Forme réelle renvoyée par le site pour JEI.
    let jei = versions(&["1.21", "Client", "1.21.1", "NeoForge", "Server"]);
    assert!(compatible(&jei, "1.21.1", "neoforge"));
    assert!(!compatible(&jei, "1.21.1", "fabric"));
    assert!(!compatible(&jei, "1.20.1", "neoforge"));
}

#[test]
fn un_fichier_sans_chargeur_declare_est_accepte() {
    // Mods publiés avant que CurseForge n'étiquette le chargeur : les
    // écarter reviendrait à les rendre introuvables.
    let vieux = versions(&["1.21.1", "Client"]);
    assert!(compatible(&vieux, "1.21.1", "neoforge"));
}

#[test]
fn le_bon_chargeur_est_exige_quand_il_est_declare() {
    let fabric = versions(&["1.21.1", "Fabric"]);
    assert!(!compatible(&fabric, "1.21.1", "neoforge"));
    assert!(compatible(&fabric, "1.21.1", "fabric"));
}

#[test]
fn l_url_passe_par_la_route_du_site() {
    // Et non par une URL de CDN reconstruite, qui contournerait le refus
    // éventuel de l'auteur.
    let url = download_url(238222, 8886909);
    assert!(url.starts_with("https://www.curseforge.com/api/v1/"));
    assert!(url.ends_with("/mods/238222/files/8886909/download"));
    assert!(!url.contains("forgecdn"));
}

#[test]
fn canaux() {
    assert_eq!(channel_of(1), Channel::Release);
    assert_eq!(channel_of(2), Channel::Beta);
    assert_eq!(channel_of(3), Channel::Alpha);
}
