# Lancer le jeu, et le runtime Java

[← README](../README.md)

```bash
mc-pack launch --pseudo Sam
mc-pack launch --pseudo Sam --serveur mc.exemple.fr:25565
mc-pack launch --pseudo Sam --afficher   # montre sans lancer
```

`launch` ne réinstalle rien : il suppose l'installation faite et le dit si un
fichier manque. **En ligne de commande, installer et jouer restent deux
gestes** — c'est un outil d'outilleur, et enchaîner les deux ferait attendre
huit cents mégaoctets à qui voulait seulement lancer une partie.

**Dans la fenêtre, il n'y en a plus qu'un**, et ce n'est pas une contradiction :
c'est la résolution du même motif. Le launcher compare l'empreinte du verrou
publié à celle du verrou posé — une requête, quelques kilooctets — et n'a donc
plus besoin de faire le choix à l'aveugle. Le bouton dit INSTALLER quand rien
n'est en place, JOUER ensuite, et rattrape de lui-même ce qui a bougé. Voir
`mc_pack::jeu::enchainement` et [interface.md](interface.md).

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

Minecraft 1.21.1 exige Java 21, et refuse de démarrer sous une majeure antérieure. Un joueur n'a aucune raison
d'en avoir un, et celui qu'il a est souvent un 8 ou un 17 laissé par un vieux
modpack.

La majeure exigée est **écrite dans le verrou**, et vérifiée à chaque
lancement. C'est une égalité et non un minimum : un Java plus récent que celui
avec lequel NeoForge a été installé change le comportement des mixins et le
format des registres, et le serveur tranche par une éjection qui ne nomme pas
sa cause. Un poste qui a un Java 22 et un pack qui demande 21 recevra donc un
Temurin 21 dédié, sans qu'on touche au 22 du système.

Le launcher ne propose aucun choix de version de Java, et c'est délibéré : ce
n'est pas un réglage, c'est une propriété du pack.

```bash
cargo run -p mc-java --release            # détecte, installe au besoin
cargo run -p mc-java --release -- --check # détecte seulement, code 1 si absent
```

Les candidats sont sondés en exécutant `java -version`, jamais en lisant un
chemin. Sans résultat, un Temurin est installé depuis l'API Adoptium, son
SHA-256 vérifié, et l'installation confirmée en relançant le binaire. Ce
runtime n'est pas ajouté au `PATH` et ne touche pas au Java du système.
