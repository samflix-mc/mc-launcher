//! CurseForge : avec la clé quand elle marche, sans elle sinon.

use anyhow::Result;

use crate::resolve::Registry;
use crate::Candidate;

impl Registry {
    /// CurseForge, avec la clé si elle marche, sans elle sinon.
    ///
    /// Une clé refusée ne doit pas tout arrêter : elle expire, elle se révoque,
    /// et le mode sans clé reste capable d'installer. L'avertissement n'est émis
    /// qu'une fois par exécution — répété à chaque mod, il deviendrait du bruit
    /// qu'on cesse de lire.
    pub(crate) async fn curseforge_any(
        &self,
        id_or_slug: &str,
        mc: &str,
        loader: &str,
    ) -> Result<Vec<Candidate>> {
        if let Some(cf) = &self.curseforge
            && !self.key_rejected.load(std::sync::atomic::Ordering::Relaxed)
        {
            match cf.candidates(id_or_slug, mc, loader).await {
                Ok(found) if !found.is_empty() => return Ok(found),
                Ok(_) => {}
                Err(e) if crate::curseforge::is_key_error(&e) => {
                    self.key_rejected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("  ! {e}\n    Repli sur CurseForge sans clé.");
                }
                Err(e) => return Err(e),
            }
        }
        self.curseforge_web.candidates(id_or_slug, mc, loader).await
    }

    /// Cherche un projet par le `modId` que déclare un jar.
    pub(crate) async fn find_by_mod_id(&self, mod_id: &str, mc: &str, loader: &str) -> Result<Vec<Candidate>> {
        let found = self.modrinth.find_by_mod_id(mod_id, mc, loader).await?;
        if !found.is_empty() {
            return Ok(found);
        }
        if let Some(cf) = &self.curseforge
            && !self.key_rejected.load(std::sync::atomic::Ordering::Relaxed)
        {
            match cf.find_by_mod_id(mod_id, mc, loader).await {
                Ok(found) if !found.is_empty() => return Ok(found),
                Ok(_) => {}
                Err(e) if crate::curseforge::is_key_error(&e) => {
                    self.key_rejected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => return Err(e),
            }
        }
        // Sans clé, la recherche par mot-clé est fermée : seul un `modId` qui
        // est aussi le slug du projet peut aboutir.
        self.curseforge_web
            .find_by_mod_id(mod_id, mc, loader)
            .await
            .or_else(|_| Ok(Vec::new()))
    }

    /// Build CurseForge épinglé, avec la clé si elle marche, sans elle sinon.
    pub(crate) async fn curseforge_file(&self, id_or_slug: &str, file: &str) -> Result<Option<Candidate>> {
        if let Some(cf) = &self.curseforge
            && !self.key_rejected.load(std::sync::atomic::Ordering::Relaxed)
        {
            match cf.candidate_by_file(file).await {
                Ok(Some(found)) => return Ok(Some(found)),
                Ok(None) => {}
                Err(e) if crate::curseforge::is_key_error(&e) => {
                    self.key_rejected
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => return Err(e),
            }
        }
        self.curseforge_web
            .candidate_by_file(id_or_slug, file)
            .await
    }
}
