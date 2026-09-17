//! Une fausse Modrinth, et de vrais jars.
//!
//! La résolution croise trois sources : ce que le manifeste demande, ce que
//! l'API déclare, et ce que le jar exige. La troisième est celle qui décide —
//! c'est elle qui rattrape les dépendances qu'aucun auteur n'a saisies — et
//! elle ne se vérifie qu'avec de vraies archives contenant un vrai
//! `neoforge.mods.toml`. Les deux premières demandent des réponses d'API
//! qu'on puisse fabriquer, y compris les réponses fautives.

use std::io::Write;

/// Un jar de mod : ce qu'il déclare fournir, ce qu'il exige.
pub(crate) fn jar(mod_id: &str, exige: &[(&str, &str)]) -> Vec<u8> {
    let mut toml = format!(
        "modLoader=\"javafml\"\nloaderVersion=\"[1,)\"\nlicense=\"MIT\"\n\n\
         [[mods]]\nmodId=\"{mod_id}\"\nversion=\"1.0\"\n"
    );
    for (id, cote) in exige {
        toml.push_str(&format!(
            "\n[[dependencies.{mod_id}]]\nmodId=\"{id}\"\ntype=\"required\"\n\
             versionRange=\"[1,)\"\nside=\"{cote}\"\n"
        ));
    }
    archive(&[("META-INF/neoforge.mods.toml", toml.as_bytes())])
}

/// Un jar qui embarque une bibliothèque par JarJar.
///
/// Ignorer les jars embarqués ferait conclure à une dépendance manquante et
/// installerait un doublon — deux versions du même mod, ce que NeoForge refuse.
pub(crate) fn jar_avec_embarque(mod_id: &str, embarque: &str) -> Vec<u8> {
    let interieur = jar(embarque, &[]);
    let toml = format!(
        "modLoader=\"javafml\"\nloaderVersion=\"[1,)\"\nlicense=\"MIT\"\n\n\
         [[mods]]\nmodId=\"{mod_id}\"\nversion=\"1.0\"\n"
    );
    let metadata = br#"{"jars":[{"path":"META-INF/jarjar/embarque.jar"}]}"#;
    archive(&[
        ("META-INF/neoforge.mods.toml", toml.as_bytes()),
        ("META-INF/jarjar/metadata.json", metadata),
        ("META-INF/jarjar/embarque.jar", &interieur),
    ])
}

pub(crate) fn archive(entrees: &[(&str, &[u8])]) -> Vec<u8> {
    let mut ecrivain = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (nom, contenu) in entrees {
        ecrivain.start_file(*nom, options).unwrap();
        ecrivain.write_all(contenu).unwrap();
    }
    ecrivain.finish().unwrap().into_inner()
}

/// Ce qu'un projet Modrinth publie, tel que l'API le rend.
pub(crate) struct Projet {
    pub(crate) slug: String,
    pub(crate) client: &'static str,
    pub(crate) serveur: &'static str,
    /// `(numéro, url du jar, sha1, taille, dépendances déclarées)`
    pub(crate) versions: Vec<Version>,
}

pub(crate) struct Version {
    pub(crate) numero: String,
    pub(crate) canal: String,
    pub(crate) url: String,
    pub(crate) sha1: String,
    pub(crate) taille: u64,
    pub(crate) declare: Vec<String>,
    pub(crate) publie: String,
}

impl Projet {
    pub(crate) fn nouveau(slug: &str) -> Projet {
        Projet {
            slug: slug.to_string(),
            client: "required",
            serveur: "required",
            versions: Vec::new(),
        }
    }

    pub(crate) fn cote(mut self, client: &'static str, serveur: &'static str) -> Projet {
        self.client = client;
        self.serveur = serveur;
        self
    }

    pub(crate) fn version(mut self, version: Version) -> Projet {
        self.versions.push(version);
        self
    }

    fn json_projet(&self) -> String {
        format!(
            r#"{{"id":"{0}-id","slug":"{0}","title":"{0}",
                 "client_side":"{1}","server_side":"{2}"}}"#,
            self.slug, self.client, self.serveur
        )
    }

    fn json_versions(&self) -> String {
        let versions: Vec<String> = self.versions.iter().map(|v| v.json(&self.slug)).collect();
        format!("[{}]", versions.join(","))
    }
}

impl Version {
    pub(crate) fn nouvelle(numero: &str, url: &str, contenu: &[u8]) -> Version {
        Version {
            numero: numero.to_string(),
            canal: "release".into(),
            url: url.to_string(),
            sha1: mc_dl::Checksum::Sha1(String::new()).of(contenu),
            taille: contenu.len() as u64,
            declare: Vec::new(),
            publie: "2026-01-01T00:00:00Z".into(),
        }
    }

    pub(crate) fn canal(mut self, canal: &str) -> Version {
        self.canal = canal.to_string();
        self
    }

    pub(crate) fn publie(mut self, quand: &str) -> Version {
        self.publie = quand.to_string();
        self
    }

    /// Dépendance obligatoire annoncée par l'API — par opposition à celle que
    /// seul le jar déclare.
    pub(crate) fn declare(mut self, projet: &str) -> Version {
        self.declare.push(format!("{projet}-id"));
        self
    }

    fn json(&self, slug: &str) -> String {
        let deps: Vec<String> = self
            .declare
            .iter()
            .map(|id| {
                format!(r#"{{"project_id":"{id}","version_id":null,"dependency_type":"required"}}"#)
            })
            .collect();
        format!(
            r#"{{"id":"{slug}-{0}","project_id":"{slug}-id","name":"{slug} {0}",
                 "version_number":"{0}","version_type":"{1}","date_published":"{2}",
                 "files":[{{"url":"{3}","filename":"{slug}-{0}.jar","primary":true,
                            "size":{4},"hashes":{{"sha1":"{5}","sha512":null}}}}],
                 "dependencies":[{6}]}}"#,
            self.numero,
            self.canal,
            self.publie,
            self.url,
            self.taille,
            self.sha1,
            deps.join(",")
        )
    }
}

/// Publie un projet sur le serveur de test, aux routes que Modrinth expose.
pub(crate) fn publier(serveur: &mc_essais::Serveur, projet: &Projet) {
    serveur.json(&format!("/project/{}", projet.slug), &projet.json_projet());
    serveur.json(
        &format!("/project/{}-id", projet.slug),
        &projet.json_projet(),
    );
    serveur.json(
        &format!("/project/{}-id/version", projet.slug),
        &projet.json_versions(),
    );
    // La route d'une version isolée : c'est par elle que passe un build
    // épinglé, et c'est aussi ce que produit le rattrapage, qui désigne le
    // build qu'il vient de trouver plutôt que de rouvrir le choix.
    for version in &projet.versions {
        serveur.json(
            &format!("/version/{}-{}", projet.slug, version.numero),
            &version.json(&projet.slug),
        );
    }
}

/// Un répertoire de travail propre, effacé à la destruction.
pub(crate) struct Atelier {
    pub(crate) racine: std::path::PathBuf,
}

impl Atelier {
    pub(crate) fn neuf(nom: &str) -> Atelier {
        let racine = std::env::temp_dir().join(format!(
            "mc-mods-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Atelier { racine }
    }
}

impl Drop for Atelier {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}
