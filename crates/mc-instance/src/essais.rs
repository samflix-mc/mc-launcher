//! Une installation factice sur le disque, pour éprouver ce qui la lit.
//!
//! L'assemblage de la ligne de commande, la vérification et le classpath ne se
//! contentent pas de structures en mémoire : ils lisent des `version.json`,
//! exigent que les jars existent, et refusent de continuer si l'un manque.
//! C'est précisément ce refus qu'on veut vérifier, donc il faut de vrais
//! fichiers.
//!
//! L'arbre vit dans le répertoire temporaire, porte le numéro du processus et
//! celui du fil, et disparaît à la destruction — deux tests qui tournent en
//! parallèle ne se marchent pas dessus.

use std::path::PathBuf;

pub(crate) struct Arbre {
    pub(crate) racine: PathBuf,
}

impl Arbre {
    pub(crate) fn neuf(nom: &str) -> Arbre {
        let racine = std::env::temp_dir().join(format!(
            "mc-instance-{nom}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&racine).ok();
        std::fs::create_dir_all(&racine).unwrap();
        Arbre { racine }
    }

    pub(crate) fn shared(&self) -> PathBuf {
        self.racine.join("shared")
    }

    pub(crate) fn game_dir(&self) -> PathBuf {
        self.racine.join("instance").join("minecraft")
    }

    /// Écrit `versions/<id>/<id>.json`.
    pub(crate) fn version(&self, id: &str, json: &str) -> &Self {
        self.ecrire(
            &self
                .shared()
                .join("versions")
                .join(id)
                .join(format!("{id}.json")),
            json.as_bytes(),
        );
        self
    }

    /// Écrit `versions/<id>/<id>.jar`, le client que le classpath exige.
    pub(crate) fn client(&self, id: &str) -> &Self {
        self.ecrire(
            &self
                .shared()
                .join("versions")
                .join(id)
                .join(format!("{id}.jar")),
            b"jar",
        );
        self
    }

    /// Écrit une bibliothèque, chemin relatif à `libraries/`.
    pub(crate) fn bibliotheque(&self, relatif: &str) -> &Self {
        self.ecrire(&self.shared().join("libraries").join(relatif), b"jar");
        self
    }

    /// Range un objet d'asset sous son empreinte, et rend celle-ci.
    pub(crate) fn asset(&self, contenu: &[u8]) -> String {
        let brouillon = self.racine.join("brouillon");
        std::fs::write(&brouillon, contenu).unwrap();
        let empreinte = mc_dl::sha1_of_file(&brouillon).unwrap();
        let destination = self
            .shared()
            .join("assets")
            .join("objects")
            .join(&empreinte[..2])
            .join(&empreinte);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::rename(&brouillon, &destination).unwrap();
        empreinte
    }

    /// Écrit `assets/indexes/<id>.json` désignant les empreintes données.
    pub(crate) fn index_assets(&self, id: &str, empreintes: &[String]) -> &Self {
        let objets: Vec<String> = empreintes
            .iter()
            .enumerate()
            .map(|(rang, empreinte)| format!(r#""objet{rang}":{{"hash":"{empreinte}","size":3}}"#))
            .collect();
        self.ecrire(
            &self
                .shared()
                .join("assets")
                .join("indexes")
                .join(format!("{id}.json")),
            format!(r#"{{"objects":{{{}}}}}"#, objets.join(",")).as_bytes(),
        );
        self
    }

    fn ecrire(&self, chemin: &std::path::Path, contenu: &[u8]) {
        std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
        std::fs::write(chemin, contenu).unwrap();
    }
}

impl Drop for Arbre {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.racine).ok();
    }
}

/// Un descripteur vanilla réduit à ce qui compte, sur le modèle de celui que
/// Mojang publie : une bibliothèque sans règle, une réservée à un autre
/// système, et des arguments dont certains sont conditionnels.
pub(crate) const VANILLA: &str = r#"{
  "id": "1.21.1",
  "mainClass": "net.minecraft.client.main.Main",
  "assetIndex": { "id": "17" },
  "libraries": [
    { "name": "com.google.guava:guava:32.1.2-jre",
      "downloads": { "artifact": { "path": "com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar",
                                   "sha1": "aa", "size": 1, "url": "https://exemple.invalid/g.jar" } } },
    { "name": "org.lwjgl:lwjgl:3.3.3:natives-macos",
      "rules": [ { "action": "allow", "os": { "name": "osx" } } ] }
  ],
  "arguments": {
    "jvm": [ "-Djava.library.path=${natives_directory}", "-cp", "${classpath}" ],
    "game": [
      "--username", "${auth_player_name}",
      "--uuid", "${auth_uuid}",
      { "rules": [ { "action": "allow", "features": { "is_quick_play_multiplayer": true } } ],
        "value": [ "--quickPlayMultiplayer", "${quickPlayMultiplayer}" ] },
      { "rules": [ { "action": "allow", "features": { "has_custom_resolution": true } } ],
        "value": [ "--width", "${resolution_width}", "--height", "${resolution_height}" ] }
    ]
  }
}"#;

/// Sérialise les tests qui écrivent un exécutable ou qui en lancent un.
///
/// Sans cela, la suite échoue par intermittence sur `ETXTBSY` — « Text file
/// busy ». Un test écrit un script et l'exécute ; un autre, au même instant,
/// duplique le processus pour lancer `/bin/sh`. L'enfant hérite un instant du
/// descripteur d'écriture encore ouvert, et le noyau refuse d'exécuter un
/// fichier que quelqu'un tient en écriture. Rien dans le code vérifié n'est en
/// cause, et la panne ne se voit qu'à la charge : elle a mis sept machines en
/// parallèle à se déclarer.
///
/// Le verrou ferme la fenêtre des deux côtés — aucune écriture pendant qu'un
/// autre test lance un processus, et réciproquement. C'est le même garde, et
/// le même `ETXTBSY`, que mc-java tient dans son propre module d'essais.
///
/// Un verrou atomique plutôt qu'un `Mutex` : ces tests sont asynchrones, et
/// tenir un `MutexGuard` à travers un `await` est exactement ce que clippy
/// refuse — à raison, puisque rien ne garantit que la tâche reprenne sur le
/// même fil.
static ATELIER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) struct Atelier;

/// À tenir pendant qu'on fabrique un exécutable, ou qu'on en lance un.
pub(crate) fn atelier() -> Atelier {
    use std::sync::atomic::Ordering;
    while ATELIER
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        std::thread::yield_now();
    }
    Atelier
}

impl Drop for Atelier {
    fn drop(&mut self) {
        ATELIER.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Le delta d'un chargeur : il hérite du socle et remplace une bibliothèque.
pub(crate) const NEOFORGE: &str = r#"{
  "id": "neoforge-21.1.250",
  "inheritsFrom": "1.21.1",
  "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
  "libraries": [
    { "name": "com.google.guava:guava:33.0.0-jre" },
    { "name": "net.neoforged.fancymodloader:loader:4.0.24" }
  ],
  "arguments": {
    "jvm": [ "-DlibraryDirectory=${library_directory}" ],
    "game": [ "--launchTarget", "neoforgeclient" ]
  }
}"#;
