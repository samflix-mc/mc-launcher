# Lancer le jeu, et le runtime Java

[← README](../README.md)

```bash
mc-pack launch --pseudo Sam
mc-pack launch --pseudo Sam --serveur mc.exemple.fr:25565
mc-pack launch --pseudo Sam --afficher   # montre sans lancer
```

`launch` ne réinstalle rien : il suppose l'installation faite et le dit si un
fichier manque. Installer et jouer sont deux gestes distincts — les enchaîner
ferait attendre huit cents mégaoctets à qui voulait lancer une partie.

## Ce qui décide qu'un jeu démarre

Minecraft ne se lance pas, il se *compose*. Le descripteur de NeoForge n'est
qu'un delta qui désigne son socle par `inheritsFrom` ; il faut fusionner les
deux, retenir les bibliothèques valables pour ce système, assembler un
classpath et substituer une vingtaine de variables dans des arguments dont
certains n'apparaissent que sous condition. Quatre points décident que le jeu
démarre ou non :

- **l'ordre du classpath** — NeoForge remplace des bibliothèques de Mojang. La
  sienne doit passer devant, sinon la JVM charge celle du jeu et le chargeur
  échoue sur une méthode absente ;
- **le client vanilla** — NeoForge ne le déclare pas parmi ses bibliothèques ;
  il est ajouté au classpath et c'est FML qui le transforme au chargement ;
- **les natives** — inutile de les extraire. Les arguments de Mojang passent
  `org.lwjgl.system.SharedLibraryExtractPath`, et LWJGL 3.3 sort lui-même ses
  binaires des jars du classpath ; il suffit que le répertoire existe ;
- **les drapeaux** — `--quickPlayMultiplayer` n'existe dans le descripteur que
  derrière une règle `is_quick_play_multiplayer`. Ignorer ces règles produit
  une ligne de commande que le jeu refuse.

La session est celle que demande la ligne de commande : le compte Microsoft
enregistré, ou un profil hors-ligne avec `--pseudo`. Dans ce second cas l'UUID
suit la règle du serveur vanilla, donc le joueur garde le même d'une partie à
l'autre, et les backends du réseau tournent en `online-mode=false`. Voir
[l'authentification](authentification.md).

## Le runtime Java

Minecraft 1.21.1 refuse de démarrer sous Java 21. Un joueur n'a aucune raison
d'en avoir un, et celui qu'il a est souvent un 8 ou un 17 laissé par un vieux
modpack.

```bash
cargo run -p mc-java --release            # détecte, installe au besoin
cargo run -p mc-java --release -- --check # détecte seulement, code 1 si absent
```

Les candidats sont sondés en exécutant `java -version`, jamais en lisant un
chemin. Sans résultat, un Temurin est installé depuis l'API Adoptium, son
SHA-256 vérifié, et l'installation confirmée en relançant le binaire. Ce
runtime n'est pas ajouté au `PATH` et ne touche pas au Java du système.
