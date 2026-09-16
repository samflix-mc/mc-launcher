//! Rend `SAMFLIX_ENV` visible à la compilation, et recompile quand il change.
//!
//! `option_env!` est évalué une fois, à la compilation. Sans cette directive,
//! cargo garderait en cache un binaire portant l'ancienne valeur : un build de
//! préproduction relancé avec `SAMFLIX_ENV=production` continuerait d'annoncer
//! « preproduction », et le tableau de bord mentirait sans que rien ne le
//! signale.

fn main() {
    println!("cargo:rerun-if-env-changed=SAMFLIX_ENV");
}
