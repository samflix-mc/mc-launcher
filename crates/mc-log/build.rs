//! Rend `SAMFLIX_ENV` visible à la compilation, et recompile quand il change.
//!
//! `option_env!` est évalué une fois, à la compilation. Sans cette directive,
//! cargo garderait en cache un binaire portant l'ancienne valeur : un build de
//! préproduction relancé avec `SAMFLIX_ENV=production` continuerait d'annoncer
//! « preproduction », et le tableau de bord mentirait sans que rien ne le
//! signale.
//!
//! ## La garde, et pourquoi elle est ici
//!
//! Un `SAMFLIX_ENV` absent ne casse rien : la résolution retombe sur « local »,
//! le défaut prudent. C'est exactement ce qui rend l'oubli dangereux dans une
//! release — le binaire part, s'installe, se lance, et va chercher le pack de
//! développement en croyant bien faire. Rien ne le dit, puisque le défaut est
//! le silence.
//!
//! `MC_EXIGER_ENV=1` renverse ce défaut : là où elle est posée — les quatre
//! jambes Tauri de release.yml — un environnement manquant arrête la
//! compilation au lieu de la laisser produire un binaire qui ment. Elle n'est
//! posée nulle part ailleurs : sur un poste de développement, compiler sans y
//! penser doit rester possible.
//!
//! La garde vit dans ce build.rs et non dans un `if` du workflow parce que
//! c'est ici qu'on sait ce que le binaire portera réellement — un workflow ne
//! voit qu'une variable, pas le résultat de sa propagation à travers un
//! `uses:`.

fn main() {
    println!("cargo:rerun-if-env-changed=SAMFLIX_ENV");
    // Sa propre ligne, et c'est le détail sans lequel la garde manquerait le
    // build qu'elle devait arrêter : cargo ne rejoue un script de build que si
    // l'une des variables DÉCLARÉES a changé. Poser MC_EXIGER_ENV sans la
    // déclarer laisserait un script en cache — celui d'un build précédent, qui
    // n'a rien vérifié — décider à sa place.
    println!("cargo:rerun-if-env-changed=MC_EXIGER_ENV");

    let exigee = std::env::var("MC_EXIGER_ENV")
        .map(|valeur| !valeur.trim().is_empty() && valeur != "0")
        .unwrap_or(false);

    if !exigee {
        return;
    }

    let declaree = std::env::var("SAMFLIX_ENV")
        .map(|valeur| !valeur.trim().is_empty())
        .unwrap_or(false);

    if !declaree {
        // `cargo::error` et non un panic : le message sort formaté comme une
        // erreur de compilation, à sa place dans le journal du job, au lieu
        // d'une trace de panique qu'on lit comme un bogue de l'outillage.
        println!(
            "cargo::error=MC_EXIGER_ENV est posée mais SAMFLIX_ENV ne l'est pas. \
             Le binaire produit se croirait « local » : il irait chercher le pack \
             de développement et ferait entrer les joueurs sur le serveur de dev. \
             Poser SAMFLIX_ENV, ou retirer MC_EXIGER_ENV si la compilation n'est \
             pas destinée à être publiée."
        );
    }
}
