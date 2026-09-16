//! Ce qu'un manifeste doit respecter pour être installable.

use anyhow::{bail, Result};

use super::{Manifest, SCHEMA};

impl Manifest {
    pub(super) fn check(&self) -> Result<()> {
        if self.schema != SCHEMA {
            bail!(
                "manifeste au format {} alors que cette version lit le format {SCHEMA}",
                self.schema
            );
        }
        if self.loader.kind != "neoforge" {
            bail!(
                "seul le chargeur neoforge est géré, manifeste : « {} »",
                self.loader.kind
            );
        }
        // « name » désigne un répertoire d'instance : il est joint à la racine
        // des données, et « deploy » supprime ensuite tous les .jar qu'il
        // trouve dans le dossier obtenu. Tant que le manifeste venait d'un
        // fichier qu'on édite soi-même, le champ était de confiance ; depuis
        // que le pack distant est la source par défaut, il vient du réseau.
        //
        // Un « .. » remonte, et un chemin absolu fait mieux : PathBuf::join
        // écarte purement et simplement le préfixe. Un hôte de pack compromis
        // ou une faute de frappe obtiendrait alors une suppression de .jar et
        // une écriture de fichiers là où il veut, à chaque installation.
        //
        // On ne refuse que ce qui sort du répertoire. Pas de liste blanche de
        // caractères : refuser ici ce qui est seulement inhabituel
        // condamnerait un pack futur chez tous les launchers déjà distribués,
        // et un launcher qui refuse le pack ne peut plus se dépanner — il
        // faudrait télécharger le pack qu'il refuse de lire.
        let nom = self.name.trim();
        if nom.is_empty() || nom == "." || nom == ".." || nom.contains('/') || nom.contains('\\') {
            bail!(
                "« {} » ne peut pas nommer un répertoire d'instance : le pack \
                 s'installerait hors de la racine des données",
                self.name
            );
        }

        let mut seen = std::collections::BTreeSet::new();
        for entry in &self.mods {
            if !seen.insert(entry.slug.to_ascii_lowercase()) {
                bail!("{} apparaît deux fois dans le manifeste", entry.slug);
            }
            if entry.file.is_some() && entry.version.is_some() {
                bail!(
                    "{} épingle à la fois un build (file) et un numéro de version : \
                     les deux se contrediraient",
                    entry.slug
                );
            }
        }

        // Les clés de « servers » ne sont pas contrôlées ici : voir
        // problemes_de_serveurs, et la raison pour laquelle ce contrôle-là ne
        // peut pas vivre dans check.
        Ok(())
    }
}

#[cfg(test)]
#[path = "controle.test.rs"]
mod tests;
