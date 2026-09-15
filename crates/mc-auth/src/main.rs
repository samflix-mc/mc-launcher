//! Vérifie de bout en bout qu'un Client ID Azure est utilisable pour Minecraft.
//!
//!     mc-auth <CLIENT_ID>
//!
//! Tant que Microsoft n'a pas approuvé l'application, l'appel à
//! api.minecraftservices.com répond 403 — et c'est précisément cette tentative
//! qui doit avoir eu lieu avant de soumettre https://aka.ms/mce-reviewappid.

use anyhow::{Result, bail};
use mc_auth::Auth;

#[tokio::main]
async fn main() -> Result<()> {
    let Some(client_id) = std::env::args().nth(1).or_else(|| std::env::var("AZURE_CLIENT_ID").ok())
    else {
        bail!("usage : mc-auth <CLIENT_ID>  (ou variable AZURE_CLIENT_ID)");
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
    println!("  refresh token : {}", if session.refresh_token.is_some() { "reçu" } else { "absent" });
    Ok(())
}
