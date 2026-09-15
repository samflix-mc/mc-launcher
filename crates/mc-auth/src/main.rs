//! Vérifie de bout en bout qu'un Client ID Azure est utilisable pour Minecraft.
//!
//!     mc-auth <CLIENT_ID>        chaîne complète, exige une application approuvée
//!     mc-auth --offline <PSEUDO>  profil local, pour développer sans Microsoft
//!
//! Tant que Microsoft n'a pas approuvé l'application, l'appel à
//! api.minecraftservices.com répond 403 — et c'est précisément cette tentative
//! qui doit avoir eu lieu avant de soumettre https://aka.ms/mce-reviewappid.

use anyhow::{Result, bail};
use mc_auth::{Auth, offline_session};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Mode hors-ligne : produit un profil sans contacter Microsoft, pour
    // développer le reste du launcher pendant que l'inscription d'application
    // attend son approbation. Ne permet de rejoindre qu'un serveur en
    // online-mode=false — ce qui est le cas des backends du réseau, puisque
    // l'authentification y est déléguée au proxy.
    if args.first().map(String::as_str) == Some("--offline") {
        let Some(name) = args.get(1) else {
            bail!("usage : mc-auth --offline <PSEUDO>");
        };
        let s = offline_session(name);
        println!("Profil hors-ligne — aucun jeton, serveur online-mode=false uniquement.");
        println!("  pseudo : {}", s.profile.name);
        println!("  uuid   : {}", s.profile.id);
        return Ok(());
    }

    let Some(client_id) = args
        .into_iter()
        .next()
        .or_else(|| std::env::var("AZURE_CLIENT_ID").ok())
    else {
        bail!("usage : mc-auth <CLIENT_ID>  |  mc-auth --offline <PSEUDO>");
    };

    let auth = Auth::new(&client_id)?;
    let session = auth
        .login(|dc| {
            println!("\n  Ouvre {}", dc.verification_uri);
            println!("  et saisis le code : {}\n", dc.user_code);
            println!("  (en attente de la validation…)");
        })
        .await?;

    println!("\nConnecté.");
    println!("  pseudo : {}", session.profile.name);
    println!("  uuid   : {}", session.profile.id);
    println!(
        "  refresh token : {}",
        if session.refresh_token.is_some() {
            "reçu"
        } else {
            "absent"
        }
    );
    Ok(())
}
