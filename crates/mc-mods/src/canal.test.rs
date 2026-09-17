use super::Channel;

#[test]
fn un_canal_plus_stable_que_la_limite_est_accepte() {
    assert!(Channel::Release.allowed_by(Channel::Beta));
    assert!(Channel::Beta.allowed_by(Channel::Beta));
    assert!(!Channel::Alpha.allowed_by(Channel::Beta));
    assert!(!Channel::Beta.allowed_by(Channel::Release));
}

/// Ces trois mots viennent des manifestes et des réponses d'API. Confondre
/// « beta » avec le défaut le ferait lire comme une alpha : un pack qui
/// s'autorise les bêtas n'en recevrait plus, et un pack qui s'arrête aux
/// releases laisserait entrer des alphas. Rien ne casserait — la liste des
/// mods serait simplement fausse.
#[test]
fn chaque_canal_se_lit_sous_le_nom_que_les_manifestes_emploient() {
    assert_eq!(Channel::parse("release"), Channel::Release);
    assert_eq!(Channel::parse("beta"), Channel::Beta);
    assert_eq!(Channel::parse("alpha"), Channel::Alpha);

    // La casse et les espaces ne décident pas : les deux varient d'une source
    // à l'autre.
    assert_eq!(Channel::parse(" BETA "), Channel::Beta);
    assert_eq!(Channel::parse("Release"), Channel::Release);

    // Ce qu'on ne sait pas lire est traité comme le moins stable : mieux vaut
    // écarter par prudence que promouvoir par ignorance.
    assert_eq!(Channel::parse("nightly"), Channel::Alpha);
    assert_eq!(Channel::parse(""), Channel::Alpha);
}

/// L'écriture est l'inverse de la lecture, et ces chaînes sont celles qui
/// finissent dans le verrou : les changer rendrait illisible un verrou déjà
/// écrit.
#[test]
fn chaque_canal_s_ecrit_comme_il_se_relit() {
    for canal in [Channel::Release, Channel::Beta, Channel::Alpha] {
        assert_eq!(Channel::parse(canal.as_str()), canal, "pour {canal:?}");
    }
    assert_eq!(Channel::Release.as_str(), "release");
    assert_eq!(Channel::Beta.as_str(), "beta");
    assert_eq!(Channel::Alpha.as_str(), "alpha");
}
