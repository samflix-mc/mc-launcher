use super::{Reponse, analyser, longueur_annoncee, rendre};

const REQUETE: &str = "POST /commande/statut HTTP/1.1\r\n\
Host: 127.0.0.1:1421\r\n\
Content-Type: application/json\r\n\
Content-Length: 2\r\n\
\r\n\
{}";

#[test]
fn une_requete_ordinaire_se_lit() {
    let (methode, chemin, entetes) = analyser(REQUETE).expect("requête lisible");

    assert_eq!(methode, "POST");
    assert_eq!(chemin, "/commande/statut");
    assert_eq!(entetes.get("content-length").map(String::as_str), Some("2"));
    assert_eq!(longueur_annoncee(&entetes), 2);
}

/// Les noms d'en-tête ne sont pas sensibles à la casse, et les clients ne s'en
/// privent pas. Les comparer tels quels ferait rater `Content-Length` écrit
/// autrement — et le corps serait alors tronqué à zéro octet.
#[test]
fn les_entetes_se_lisent_quelle_que_soit_leur_casse() {
    let brut = "POST /x HTTP/1.1\r\nCONTENT-LENGTH: 17\r\n\r\n";
    let (_, _, entetes) = analyser(brut).expect("requête lisible");

    assert_eq!(longueur_annoncee(&entetes), 17);
}

/// La chaîne de requête ne fait pas partie du routage : `/commande/statut?t=1`
/// désigne la même commande, et la garder produirait une commande inconnue.
#[test]
fn la_chaine_de_requete_ne_compte_pas_dans_le_chemin() {
    let (_, chemin, _) = analyser("GET /evenements?depuis=3 HTTP/1.1\r\n\r\n").expect("lisible");

    assert_eq!(chemin, "/evenements");
}

/// Sans en-tête de longueur, on n'attend AUCUN octet de corps.
///
/// Le contraire — attendre « ce qui vient » — ferait pendre la connexion
/// jusqu'à ce que le client abandonne, sur toute requête sans corps.
#[test]
fn sans_longueur_annoncee_on_n_attend_rien() {
    let (_, _, entetes) = analyser("GET / HTTP/1.1\r\nHost: x\r\n\r\n").expect("lisible");

    assert_eq!(longueur_annoncee(&entetes), 0);
}

/// Une longueur qui n'est pas un nombre vaut zéro, et ne panique pas : la
/// requête vient d'un client qu'on ne contrôle pas.
#[test]
fn une_longueur_illisible_vaut_zero() {
    let (_, _, entetes) =
        analyser("POST /x HTTP/1.1\r\nContent-Length: beaucoup\r\n\r\n").expect("lisible");

    assert_eq!(longueur_annoncee(&entetes), 0);
}

#[test]
fn une_requete_vide_ne_se_lit_pas() {
    assert!(analyser("").is_none());
    assert!(analyser("GET\r\n\r\n").is_none());
}

/// **Les en-têtes d'origine croisée sont sur TOUTE réponse.**
///
/// Le front tourne sur le serveur de développement d'Angular, donc sur un
/// autre port : sans eux, le navigateur refuse la réponse sans que rien
/// n'apparaisse côté serveur — on voit une requête réussie et un front qui ne
/// reçoit rien.
#[test]
fn toute_reponse_porte_les_entetes_d_origine_croisee() {
    let texte = rendre(&Reponse::json("{}".to_string()));

    assert!(texte.contains("Access-Control-Allow-Origin: *"), "{texte}");
    assert!(texte.contains("Access-Control-Allow-Headers: Content-Type"));
}

/// La longueur annoncée est celle du corps, en OCTETS.
///
/// Les libellés du launcher sont en français : « é » pèse deux octets pour un
/// caractère, et annoncer le nombre de caractères tronquerait la réponse d'un
/// octet par accent — ce qui se voit comme un JSON invalide, au hasard.
#[test]
fn la_longueur_est_comptee_en_octets() {
    let corps = "\"prêt à jouer\"".to_string();
    let attendu = corps.len();
    let texte = rendre(&Reponse::json(corps));

    assert!(
        texte.contains(&format!("Content-Length: {attendu}")),
        "{texte}"
    );
}

/// Une erreur se sérialise comme une CHAÎNE JSON, exactement comme `invoke`
/// rejette. Le front la traite alors par le même chemin dans les deux
/// transports.
#[test]
fn une_erreur_a_la_forme_de_celle_du_pont() {
    let reponse = Reponse::erreur(404, "commande inconnue : truc");

    assert_eq!(reponse.code, 404);
    assert_eq!(reponse.corps, "\"commande inconnue : truc\"");
    assert_eq!(reponse.type_mime, "application/json");
}

#[test]
fn les_codes_portent_leur_raison() {
    assert!(rendre(&Reponse::erreur(404, "x")).starts_with("HTTP/1.1 404 Not Found"));
    assert!(rendre(&Reponse::json("1".into())).starts_with("HTTP/1.1 200 OK"));
}
