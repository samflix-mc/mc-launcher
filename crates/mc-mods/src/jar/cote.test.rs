use super::Side;

#[test]
fn union_des_cotes() {
    assert_eq!(Side::Client.union(Side::Client), Side::Client);
    assert_eq!(Side::Client.union(Side::Server), Side::Both);
    assert_eq!(Side::Both.union(Side::Client), Side::Both);
    assert!(Side::Both.includes(Side::Server));
    assert!(!Side::Client.includes(Side::Server));
}

/// Ces mots viennent des manifestes des mods et des réponses d'API : Modrinth
/// écrit « server », NeoForge « DEDICATED_SERVER », et la casse varie d'un
/// auteur à l'autre. En manquer un range le mod du mauvais côté — donc un
/// serveur qui refuse de démarrer, ou un client sans son mod.
#[test]
fn les_cotes_se_lisent_tels_que_les_manifestes_les_ecrivent() {
    assert_eq!(Side::parse("client"), Some(Side::Client));
    assert_eq!(Side::parse("CLIENT"), Some(Side::Client));
    assert_eq!(Side::parse("server"), Some(Side::Server));
    assert_eq!(Side::parse("DEDICATED_SERVER"), Some(Side::Server));
    assert_eq!(Side::parse(" both "), Some(Side::Both));

    // Ce qu'on ne sait pas lire ne se devine pas : c'est à l'appelant de
    // décider quoi faire d'un côté absent, et non à cette table d'inventer.
    assert_eq!(Side::parse("les deux"), None);
    assert_eq!(Side::parse(""), None);
}

/// L'écriture est l'inverse de la lecture, et ces chaînes finissent dans le
/// fichier de verrou : les changer rendrait illisible un verrou déjà écrit.
#[test]
fn chaque_cote_s_ecrit_comme_il_se_relit() {
    for cote in [Side::Client, Side::Server, Side::Both] {
        assert_eq!(Side::parse(cote.as_str()), Some(cote), "pour {cote:?}");
    }
    assert_eq!(Side::Client.as_str(), "client");
    assert_eq!(Side::Server.as_str(), "server");
    assert_eq!(Side::Both.as_str(), "both");
}
