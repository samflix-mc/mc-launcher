//! A fake Modrinth, and real jars.
//!
//! Resolution cross-checks three sources: what the manifest asks for, what
//! the API declares, and what the jar requires. The third is the one that
//! decides — it's the one that catches the dependencies no author entered —
//! and it can only be verified with real archives containing a real
//! `neoforge.mods.toml`. The first two need API responses we can fabricate,
//! including the faulty ones.

use std::io::Write;

/// A mod jar: what it declares it provides, what it requires.
pub(crate) fn jar(mod_id: &str, requires: &[(&str, &str)]) -> Vec<u8> {
    let mut toml = format!(
        "modLoader=\"javafml\"\nloaderVersion=\"[1,)\"\nlicense=\"MIT\"\n\n\
         [[mods]]\nmodId=\"{mod_id}\"\nversion=\"1.0\"\n"
    );
    for (id, side) in requires {
        toml.push_str(&format!(
            "\n[[dependencies.{mod_id}]]\nmodId=\"{id}\"\ntype=\"required\"\n\
             versionRange=\"[1,)\"\nside=\"{side}\"\n"
        ));
    }
    archive(&[("META-INF/neoforge.mods.toml", toml.as_bytes())])
}

/// A jar that bundles a library via JarJar.
///
/// Ignoring bundled jars would conclude a dependency is missing and install
/// a duplicate — two versions of the same mod, which NeoForge refuses.
pub(crate) fn jar_with_bundled(mod_id: &str, bundled: &str) -> Vec<u8> {
    let inner = jar(bundled, &[]);
    let toml = format!(
        "modLoader=\"javafml\"\nloaderVersion=\"[1,)\"\nlicense=\"MIT\"\n\n\
         [[mods]]\nmodId=\"{mod_id}\"\nversion=\"1.0\"\n"
    );
    let metadata = br#"{"jars":[{"path":"META-INF/jarjar/bundled.jar"}]}"#;
    archive(&[
        ("META-INF/neoforge.mods.toml", toml.as_bytes()),
        ("META-INF/jarjar/metadata.json", metadata),
        ("META-INF/jarjar/bundled.jar", &inner),
    ])
}

/// A jar that bundles two libraries, like tr7zw's mods.
pub(crate) fn jar_with_two_bundled(mod_id: &str, one: &str, other: &str) -> Vec<u8> {
    let first = jar(one, &[]);
    let second = jar(other, &[]);
    let toml = format!(
        "modLoader=\"javafml\"\nloaderVersion=\"[1,)\"\nlicense=\"MIT\"\n\n\
         [[mods]]\nmodId=\"{mod_id}\"\nversion=\"1.0\"\n"
    );
    let metadata = br#"{"jars":[{"path":"META-INF/jarjar/one.jar"},
                               {"path":"META-INF/jarjar/other.jar"}]}"#;
    archive(&[
        ("META-INF/neoforge.mods.toml", toml.as_bytes()),
        ("META-INF/jarjar/metadata.json", metadata),
        ("META-INF/jarjar/one.jar", &first),
        ("META-INF/jarjar/other.jar", &second),
    ])
}

pub(crate) fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// What a Modrinth project publishes, as the API renders it.
pub(crate) struct Project {
    pub(crate) slug: String,
    pub(crate) client: &'static str,
    pub(crate) server: &'static str,
    /// `(number, jar url, sha1, size, declared dependencies)`
    pub(crate) versions: Vec<Version>,
}

pub(crate) struct Version {
    pub(crate) number: String,
    pub(crate) channel: String,
    pub(crate) url: String,
    pub(crate) sha1: String,
    pub(crate) size: u64,
    pub(crate) declared: Vec<(String, Option<String>)>,
    pub(crate) published: String,
}

impl Project {
    pub(crate) fn new(slug: &str) -> Project {
        Project {
            slug: slug.to_string(),
            client: "required",
            server: "required",
            versions: Vec::new(),
        }
    }

    pub(crate) fn sides(mut self, client: &'static str, server: &'static str) -> Project {
        self.client = client;
        self.server = server;
        self
    }

    pub(crate) fn version(mut self, version: Version) -> Project {
        self.versions.push(version);
        self
    }

    fn project_json(&self) -> String {
        format!(
            r#"{{"id":"{0}-id","slug":"{0}","title":"{0}",
                 "client_side":"{1}","server_side":"{2}"}}"#,
            self.slug, self.client, self.server
        )
    }

    fn versions_json(&self) -> String {
        let versions: Vec<String> = self.versions.iter().map(|v| v.json(&self.slug)).collect();
        format!("[{}]", versions.join(","))
    }
}

impl Version {
    pub(crate) fn new(number: &str, url: &str, content: &[u8]) -> Version {
        Version {
            number: number.to_string(),
            channel: "release".into(),
            url: url.to_string(),
            sha1: mc_dl::Checksum::Sha1(String::new()).of(content),
            size: content.len() as u64,
            declared: Vec::new(),
            published: "2026-01-01T00:00:00Z".into(),
        }
    }

    pub(crate) fn channel(mut self, channel: &str) -> Version {
        self.channel = channel.to_string();
        self
    }

    pub(crate) fn published(mut self, when: &str) -> Version {
        self.published = when.to_string();
        self
    }

    /// Mandatory dependency announced by the API — as opposed to one only
    /// the jar declares. No version: any will do.
    pub(crate) fn declare(mut self, project: &str) -> Version {
        self.declared.push((format!("{project}-id"), None));
        self
    }

    /// Mandatory dependency on a **precise build**.
    ///
    /// This is what Iris does for Sodium: its compatibility mixins target
    /// classes that get renamed from one version to the next, so a range
    /// wouldn't be enough.
    pub(crate) fn declare_build(mut self, project: &str, version: &str) -> Version {
        self.declared.push((
            format!("{project}-id"),
            Some(format!("{project}-{version}")),
        ));
        self
    }

    fn json(&self, slug: &str) -> String {
        let deps: Vec<String> = self
            .declared
            .iter()
            .map(|(id, build)| {
                let build = match build {
                    Some(build) => format!("\"{build}\""),
                    None => "null".to_string(),
                };
                format!(
                    r#"{{"project_id":"{id}","version_id":{build},"dependency_type":"required"}}"#
                )
            })
            .collect();
        format!(
            r#"{{"id":"{slug}-{0}","project_id":"{slug}-id","name":"{slug} {0}",
                 "version_number":"{0}","version_type":"{1}","date_published":"{2}",
                 "files":[{{"url":"{3}","filename":"{slug}-{0}.jar","primary":true,
                            "size":{4},"hashes":{{"sha1":"{5}","sha512":null}}}}],
                 "dependencies":[{6}]}}"#,
            self.number,
            self.channel,
            self.published,
            self.url,
            self.size,
            self.sha1,
            deps.join(",")
        )
    }
}

/// Publishes a project on the test server, at the routes Modrinth exposes.
pub(crate) fn publish(server: &mc_testkit::Server, project: &Project) {
    server.json(
        &format!("/project/{}", project.slug),
        &project.project_json(),
    );
    server.json(
        &format!("/project/{}-id", project.slug),
        &project.project_json(),
    );
    server.json(
        &format!("/project/{}-id/version", project.slug),
        &project.versions_json(),
    );
    // The route for a single version: this is how a pinned build is
    // reached, and it's also what catch-up produces, which names the build
    // it just found rather than reopening the choice.
    for version in &project.versions {
        server.json(
            &format!("/version/{}-{}", project.slug, version.number),
            &version.json(&project.slug),
        );
    }
}

/// A clean working directory, wiped on drop.
pub(crate) struct Workshop {
    pub(crate) root: std::path::PathBuf,
}

impl Workshop {
    pub(crate) fn new(name: &str) -> Workshop {
        let root = std::env::temp_dir().join(format!(
            "mc-mods-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        Workshop { root }
    }
}

impl Drop for Workshop {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}
