//! Les quatre commandes du binaire de session.

use anyhow::{Context, Result};

use mc_auth::{Auth, offline_session};

/// Ouvre une session et l'enregistre.
pub async fn login() -> Result<()> {
    let auth = Auth::login(|code| {
        println!("\n  Ouvre {}", code.verification_uri_directe);
        println!(
            "  (ou {} et saisis {})",
            code.verification_uri, code.user_code
        );
        println!("\n  En attente de la validation…");
    })
    .await?;

    let session = auth.session().await?;
    mc_auth::enregistrer(&auth.etat().await?)?;

    println!("\nConnecté.");
    afficher(&session);
    if !auth.owns_game().await? {
        println!("\n  ⚠ ce compte ne possède pas Minecraft Java Edition.");
    }
    Ok(())
}

/// Affiche la session enregistrée, en la rafraîchissant si besoin.
pub async fn whoami() -> Result<()> {
    let Some(etat) = mc_auth::charger() else {
        println!("Aucune session — lancer « mc-auth login ».");
        return Ok(());
    };

    let auth = Auth::resume(&etat)?;
    let session = auth
        .session()
        .await
        .context("la session enregistrée n'est plus valable — relancer « mc-auth login »")?;

    // Le rafraîchissement est paresseux : ce que l'appel ci-dessus a renouvelé
    // serait perdu sans cette réécriture, et le lancement suivant repartirait
    // d'un jeton périmé.
    mc_auth::enregistrer(&auth.etat().await?)?;

    println!("Session enregistrée dans {}", mc_auth::chemin().display());
    afficher(&session);
    Ok(())
}

/// Oublie la session.
pub fn logout() -> Result<()> {
    mc_auth::effacer()?;
    println!("Session oubliée.");
    Ok(())
}

/// Profil local, sans contacter personne.
pub fn hors_ligne(pseudo: &str) {
    let session = offline_session(pseudo);
    println!("Profil hors-ligne — aucun jeton, serveur online-mode=false uniquement.");
    afficher(&session);
}

fn afficher(session: &mc_auth::Session) {
    println!("  pseudo : {}", session.profile.name);
    println!("  uuid   : {}", session.profile.id);
}

#[cfg(test)]
#[path = "commandes.test.rs"]
mod tests;
