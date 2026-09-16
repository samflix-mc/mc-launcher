# mc-launcher

Launcher de bureau pour un réseau Minecraft 1.21.1 / NeoForge.

| brique | état |
|---|---|
| `crates/mc-auth` — authentification Microsoft | écrite, en attente de l'approbation Azure |
| `crates/mc-log` — journaux et incidents | écrite |
| `crates/mc-java` — runtime Java | écrite |
| `crates/mc-mods` — résolution des mods | écrite |
| `crates/mc-instance` — jeu et chargeur | écrite |
| `crates/mc-pack` — manifeste et installation | écrite |
| ligne de commande JVM + Quick Play | écrite |
| interface | à faire |

## Installer un pack

```bash
cargo run -p mc-pack --release -- install                     # le pack publié
cargo run -p mc-pack --release -- verify [source] [--deep]
cargo run -p mc-pack --release -- launch [source] --pseudo Sam

# faire bouger le pack : le manifeste vit dans mc-content
cargo run -p mc-pack --release -- lock ../mc-content/launcher/samflix.json
```

### D'où vient la liste des mods

Elle n'est pas décidée ici, et ce dépôt n'en garde aucune copie — deux
inventaires finissent toujours par diverger, et on découvre lequel avait raison
en production. Le pack est publié par
[mc-content](https://github.com/samflix-mc/mc-content), aux côtés de ce que
reçoivent les serveurs, et servi en HTTPS par
[mc-launcher-site](https://github.com/samflix-mc/mc-launcher-site) :

| environnement du binaire | pack téléchargé |
|---|---|
| `production` — publié sur tag | `mc-launcher.ggy.info` |
| `preproduction` — construit sur `main` | `mc-launcher-staging.ggy.info` |
| `development` — construit ailleurs | `mc-launcher-dev.ggy.info` |
| `local` — compilé à la main | `mc-launcher-dev.ggy.info` |

**Le binaire sait d'où il vient**, et c'est cela qui choisit le pack. L'adresse
était auparavant écrite en dur sur la production : une préproduction
téléchargeait le pack des joueurs, et n'éprouvait donc rien de ce qu'elle était
censée éprouver.

L'environnement est déclaré, jamais déduit, et dans cet ordre :

1. `SAMFLIX_ENV` **au lancement** — prioritaire sur tout le reste ;
2. `SAMFLIX_ENV` **figé à la compilation**, ce que pose la CI ;
3. `local` à défaut.

Le premier point vaut d'être connu dans les deux sens : il permet de rejouer un
binaire de production contre le pack de dev sans recompiler, et il explique
qu'un shell où la variable traîne change la cible sans rien annoncer.

Un binaire compilé à la main vise la dev, et c'est le moins coûteux des deux
choix : personne ne compile ce launcher pour jouer, tandis qu'un binaire de
travail qui installerait le pack des joueurs serait difficile à remarquer.
`[source]` reste prioritaire sur tout.

Le manifeste déclare aussi **où se connecter**, par environnement — c'est le
même fichier partout, servi sous trois noms, donc c'est au client de choisir.
`launch` rejoint donc le bon serveur sans qu'on ait à le nommer. La
préproduction n'y figure pas : elle n'a pas de serveurs Minecraft derrière elle,
et le jeu s'y ouvre sur le menu.

La raison de tout cela tient en une ligne :
**le client et les serveurs doivent charger les mêmes builds.** Les registres
NeoForge sont négociés à la connexion ; un mod en version différente d'un côté
éjecte le joueur, sans message exploitable. Tenir deux inventaires — l'un pour
le launcher, l'autre pour les serveurs — c'est accepter qu'ils divergent un
jour, et découvrir lequel a raison en production.

Un chemin local reste accepté : c'est ainsi qu'on fait bouger le pack, en
pointant `lock` sur le manifeste de mc-content. La différence entre les deux
n'est pas cosmétique :

| source | versions | verrou |
|---|---|---|
| un chemin | résolues à chaque passage | réécrit à côté du manifeste |
| une URL | **rejouées depuis le verrou publié** | téléchargé, jamais recalculé |

Un joueur ne résout rien. S'il le faisait, sa machine choisirait ses propres
versions le jour où un mod en publie une nouvelle — exactement la divergence
qu'on cherche à éviter. `lock` refuse donc une URL : un pack publié arrive déjà
verrouillé, c'est mc-content qui l'a résolu.

Chaque téléchargement réussi laisse une copie dans `cache/packs/<hôte>/`. Sans
réseau, cette copie prend le relais et l'utilisateur en est averti — jouer avec
le pack d'hier vaut mieux que ne pas jouer. Le rangement par hôte n'est pas un
détail : les manifestes de dev et de production portent le même nom de fichier,
et à plat une panne de réseau ressortirait le pack du mauvais environnement.

`launch` et `verify` ne touchent jamais au réseau : tous deux parlent de
l'installation posée sur le disque. Seul `install` rafraîchit la copie locale.

L'ordre des étapes découle des dépendances entre elles : la version du
chargeur est résolue d'abord pour que le verrou consigne un numéro et non le
mot `latest` ; les fichiers de Mojang donnent au passage la version de Java
exigée ; l'installateur NeoForge patche le client vanilla et a besoin de ce
Java ; les mods viennent ensuite ; le verrou est écrit en dernier, puisqu'il
décrit ce qui a réellement été fait.

### Le manifeste

```json
{
  "schema": 1,
  "name": "samflix",
  "minecraft": "1.21.1",
  "loader": { "type": "neoforge", "version": "latest" },
  "mods": [
    { "slug": "jei" },
    { "slug": "jade", "file": "eYz2YBGT" },
    { "slug": "embeddium", "side": "client", "channel": "beta" }
  ]
}
```

Seul `slug` est obligatoire ; chacun des autres champs consigne une décision.
`file` fige un build exact — c'est ce qui rend une installation reproductible —
`source` impose Modrinth ou CurseForge, `side` contredit ce que la plateforme
annonce, `channel` autorise une préversion.

### Le verrou

Le verrou est écrit à côté du manifeste et se versionne avec lui, dans
mc-content. Le manifeste dit ce qu'on veut, le verrou dit ce qu'on a eu : la version
retenue quand aucune n'était imposée, et **les dépendances ajoutées
d'elles-mêmes**, chacune avec la raison de sa présence. `install --locked` le
rejoue à l'identique des mois plus tard.

Chaque entrée porte son URL de téléchargement, ce qui rend le verrou lisible
par autre chose que le launcher : la CI de mc-content vérifie que chacune
répond, et un serveur peut installer le jar sans rien savoir de Modrinth.

Rejouer un verrou ne le réécrit pas. Le régénérer effacerait la colonne
`reason` — tout y deviendrait « demandé par le manifeste », puisque c'est le
verrou lui-même qui a dicté les demandes — et on perdrait la seule trace de ce
qui n'avait jamais été demandé.

## Où vivent les fichiers

```
~/.local/share/samflix-mc/
  shared/            versions/, libraries/, assets/ — forme d'un .minecraft
  instances/<nom>/minecraft/   mods, config, saves
  instances/<nom>/server/mods/ les mods du côté serveur
  runtime/temurin-21/
  cache/mods/
```

Bibliothèques et assets pèsent près d'un gigaoctet et ne dépendent que de la
version du jeu : ils sont partagés entre instances. `shared/` a la forme d'un
`.minecraft` parce que l'installateur NeoForge l'exige.

## mc-java

Minecraft 1.21.1 refuse de démarrer sous Java 21. Un joueur n'a aucune raison
d'en avoir un, et celui qu'il a est souvent un 8 ou un 17 laissé par un vieux
modpack.

```bash
cargo run -p mc-java --release            # détecte, installe au besoin
cargo run -p mc-java --release -- --check # détecte seulement
```

Les candidats sont sondés en exécutant `java -version`, jamais en lisant un
chemin. Sans résultat, un Temurin est installé depuis l'API Adoptium, son
SHA-256 vérifié, et l'installation confirmée en relançant le binaire. Ce
runtime n'est pas ajouté au `PATH` et ne touche pas au Java du système.

## mc-mods

Trois sources, essayées dans cet ordre :

| ordre | source | clé | empreinte | remarque |
|---|---|---|---|---|
| 1 | Modrinth | non | SHA-1 + SHA-512 | donne aussi la répartition client/serveur |
| 2 | CurseForge Core API | oui | SHA-1 | expose `allowModDistribution` |
| 3 | CurseForge sans clé | non | aucune | dernier recours, voir plus bas |

Modrinth passe en premier parce que son API est ouverte — rien à distribuer
avec le binaire — et qu'elle publie les empreintes. La clé CurseForge est
nominative : un launcher ne peut pas l'embarquer, elle serait extraite du
binaire et révoquée.

```bash
export CURSEFORGE_API_KEY=…            # ou ~/.config/samflix-mc/curseforge.key
cargo run -p mc-mods --example resoudre -- --detail jei jade
cargo run -p mc-mods --example resoudre -- --curseforge jade   # force la source 2 ou 3
```

Une clé refusée ne bloque pas : elle est signalée une fois, puis le troisième
chemin prend le relais. Une clé de la Core API commence par `$2a$10$` et se
génère sur console.curseforge.com — ce n'est pas un UUID.

### CurseForge sans clé

Le site web sert ses propres pages avec une API qui ne demande pas de clé.
`www.curseforge.com/api/v1/mods/{id}/files` donne la liste des fichiers avec
leur version et leur chargeur, `/files/{fid}/download` sert le jar, et
`api.cfwidget.com` fournit la seule chose que ces routes refusent : la
correspondance entre un slug et un identifiant de projet. Le téléchargement
passe par la route du site, jamais par une URL de CDN reconstruite — c'est
cette reconstruction qui contournerait le refus d'un auteur d'être redistribué.

Vérifié : un jar récupéré par ce chemin est bit-pour-bit identique à celui que
publie Modrinth pour la même version.

Ce que ce mode ne sait pas faire, et pourquoi il vient en dernier :

- **pas d'empreinte publiée.** Le SHA-1 est calculé au premier téléchargement
  et figé dans le verrou ; les installations suivantes sont vérifiées
  normalement, seule la toute première ne l'est pas. En attendant, la taille
  annoncée sert de garde-fou ;
- **pas de `allowModDistribution`.** Ce drapeau, par lequel un auteur refuse
  d'être téléchargé automatiquement par un launcher tiers, est absent de ces
  routes. Le mode avec clé l'honore ; celui-ci ne le peut pas ;
- **cinquante fichiers visibles.** La pagination est ignorée par le serveur.
  Un mod ayant publié plus de cinquante fichiers depuis sa dernière version
  compatible sort de la fenêtre — le cas est détecté et signalé, avec la
  marche à suivre, plutôt que rendu comme « introuvable » ;
- **rien n'est contractuel.** Ces routes servent le site, ne sont pas
  documentées, et cfwidget est un service tiers bénévole.

### Les dépendances qu'aucune API ne déclare

Un manifeste nomme cinq mods, le dossier `mods` en contient sept. L'écart, ce
sont les dépendances — et les fiches de publication sont souvent incomplètes,
une bibliothèque ajoutée entre deux versions n'y étant jamais reportée.

Trois sources sont croisées, de la moins à la plus fiable : ce que le manifeste
demande, ce que l'API déclare, et **ce que le jar exige** dans
`META-INF/neoforge.mods.toml` — la seule que le jeu lise. Après téléchargement,
chaque jar est ouvert, ses `modId` obligatoires comparés à ceux que le pack
fournit, et tout manque relance un tour. La boucle s'arrête quand plus rien ne
manque, soit exactement la condition que NeoForge vérifiera au démarrage.

Deux pièges méritent d'être nommés, parce qu'ils produisent des faux positifs
coûteux :

- **JarJar** — un mod embarque ses bibliothèques dans `META-INF/jarjar/`. Elles
  fournissent leur `modId` sans exister comme fichier. Les ignorer ferait
  installer un doublon, et deux versions d'un même `modId` font échouer le
  chargement ;
- **le slug n'est pas le `modId`** — `bookshelf` est publié sous
  `bookshelf-lib`. C'est le `modId` du jar qui fait foi.

Un auteur qui a désactivé la redistribution voit son refus respecté : l'URL du
CDN n'est pas reconstruite, le cas devient un message citant la page du mod.

Enfin, un même mod peut arriver deux fois par deux chemins — demandé par son
slug Modrinth, puis tiré comme dépendance par son identifiant CurseForge. Les
clés de projet diffèrent et rien ne les rapproche, sauf le `modId` que les deux
jars déclarent. Le doublon est écarté sur ce critère, en gardant le plus
vérifiable des deux : deux jars du même `modId` font échouer NeoForge au
chargement.

## Lancer le jeu

```bash
mc-pack launch --pseudo Sam
mc-pack launch --pseudo Sam --serveur mc.exemple.fr:25565
mc-pack launch --pseudo Sam --afficher   # montre sans lancer
```

`launch` ne réinstalle rien : il suppose l'installation faite et le dit si un
fichier manque. Installer et jouer sont deux gestes distincts — les enchaîner
ferait attendre huit cents mégaoctets à qui voulait lancer une partie.

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

Tant que l'application Azure n'est pas approuvée, la session est hors-ligne :
l'UUID suit la règle du serveur vanilla, donc le joueur garde le même d'une
partie à l'autre, et les backends du réseau tournent en `online-mode=false`.

## mc-log

Trois destinations, trois usages :

| destination | niveau | à quoi ça sert |
|---|---|---|
| console | `info`, réglable par `RUST_LOG` | ce qu'on regarde pendant que ça tourne |
| fichier | `debug`, rotation quotidienne, 14 jours | ce qu'on joint à un ticket |
| Sentry — *Issues* | erreurs et paniques | ce qui remonte sans qu'on ait à demander |
| Sentry — *Logs* | `info` et au-dessus | cherchable, lisible à côté de l'incident |

Un même `tracing::info!` part donc à trois endroits, et ses champs deviennent
des attributs cherchables dans Sentry. Le niveau `debug` s'arrête au fichier :
c'est plusieurs milliers de lignes par installation, qu'on lit en local.

```bash
mc-pack diagnostic                    # où sont les journaux, télémétrie active ?
mc-pack diagnostic --incident-test    # envoie un incident et confirme qu'il est parti
cargo run -p mc-log --example panique # éprouve la chaîne complète, panique comprise
RUST_LOG=mc_mods=debug mc-pack lock ../mc-content/launcher/samflix.json
```

Les journaux vivent dans `~/.local/share/samflix-mc/logs/`.

### Ce qui ne sort pas

Le launcher détient des jetons Microsoft, Xbox Live et Minecraft. La
documentation de Sentry propose `send_default_pii: true` ; **c'est le contraire
qui est fait ici**, et tout texte sortant est censuré au préalable : jetons au
format JWT, valeurs suivant un mot-clé sensible (`access_token`, `Bearer`,
`x-api-key`…), et le répertoire personnel réduit à `~`.

Trois filtres, parce que Sentry a trois canaux distincts : `before_send` pour
les incidents, `before_breadcrumb` pour les fils d'Ariane, et
**`before_send_log` pour les journaux structurés** — que les deux premiers ne
voient pas. En oublier un laisserait passer tout ce qui transite par lui.

La censure s'applique à Sentry **et au fichier de journal**. C'est délibéré :
le fichier est précisément ce qu'on demande à un joueur de coller dans un salon
Discord. Le gestionnaire de panique est remplacé pour la même raison — celui de
Rust écrit directement sur la sortie d'erreur, sans passer par `tracing`, et
laissait donc échapper un jeton présent dans un message de panique.

Ce qu'un incident emporte : version, système, composant, fil d'Ariane des
dernières opérations. Pas d'adresse IP, pas de pseudo, pas de jeton.

`SAMFLIX_TELEMETRY=0` coupe la remontée entièrement ; `SENTRY_DSN` la redirige.

### Environnements

| valeur | d'où elle vient |
|---|---|
| `local` | défaut — aucune déclaration, y compris en `--release` |
| `development` | CI, sur une branche de travail |
| `preproduction` | CI, sur `main` |
| `production` | workflow de release, sur un tag `v*` |

Un environnement décrit un **déploiement**, pas un profil de compilation. Les
déduire l'un de l'autre a un coût immédiat : `cargo run --release` sur un poste
n'a pas `debug_assertions` et se classait donc en `production` — les premiers
incidents de test du projet y sont arrivés ainsi, dans un environnement où rien
n'avait jamais été déployé.

Il est donc déclaré par `SAMFLIX_ENV`, que la CI fige à la compilation et que
le lancement peut surcharger — ce qui permet de rejouer un binaire de
production en local sans salir la production. `mc-pack diagnostic` affiche la
valeur retenue **et sa provenance**, pour qu'un environnement inattendu se
remonte à sa source sans relire le code.

Le défaut est `local` par prudence : un incident de production classé en local
se remarque, puisqu'on le cherche et qu'on ne le trouve pas. L'inverse pollue
silencieusement le seul environnement qu'on surveille vraiment.

## mc-auth

Device code Microsoft → Xbox Live → XSTS → `login_with_xbox` → licence → profil.

```bash
cargo run -p mc-auth --release -- <CLIENT_ID>        # chaîne complète
cargo run -p mc-auth --release -- --offline <PSEUDO> # profil local
```

Le *device code flow* évite une URI de redirection et un serveur HTTP local.
Tenant `consumers` obligatoire, scope `XboxLive.signin`, pas de secret client.

Les refus XSTS sont traduits : compte sans profil Xbox (2148916233), banni
(2148916227), pays non couvert (2148916235), compte enfant (2148916238).

## Mode hors-ligne

L'UUID suit la règle du serveur vanilla,
`UUID.nameUUIDFromBytes("OfflinePlayer:<pseudo>")` — UUID v3 sur MD5. Les
backends tournent en `online-mode=false` et le calculent ainsi ; un UUID
arbitraire donnerait un joueur différent à chaque connexion.

Aucun jeton produit : serveur hors-ligne uniquement.

> Ne pas emprunter le Client ID d'un launcher public. C'est l'identité d'une
> application : l'écran de consentement afficherait le nom de l'autre projet, et
> un usage inattendu ferait suspendre *son* inscription.

## Azure

| | |
|---|---|
| Client ID | `7da56647-e0e9-49d5-9aad-0c03997f3904` |
| Tenant ID | `c3fe7412-ef6b-48ab-843b-6169e3b379cf` |
| `signInAudience` | `AzureADandPersonalMicrosoftAccount` |

Le Client ID n'est pas un secret, il est destiné au binaire distribué.

```bash
az ad app create --display-name mc-launcher \
  --sign-in-audience AzureADandPersonalMicrosoftAccount \
  --is-fallback-public-client true
```

Tant que Microsoft n'a pas approuvé l'application, `login_with_xbox` répond
**403**. Cette tentative est l'activité exigée avant de soumettre
<https://aka.ms/mce-reviewappid>. Revue hebdomadaire.

## Contrôles automatiques

Trois workflows, qui se répondent sans se recouvrir.

**Contrôles** (`ci.yml`) — format, `clippy -D warnings`, tests, compilation en
release, et un appel réel à Microsoft qui doit répondre `AADSTS700038` : la
chaîne HTTP est vérifiée de bout en bout sans qu'aucun secret n'entre dans la
CI.

**Vulnérabilités** (`audit.yml`) — `cargo audit` sur les versions du verrou, à
chaque changement et tous les lundis. Le rendez-vous hebdomadaire est le plus
utile des deux : un avis RustSec paraît sur une dépendance qu'on n'a pas
touchée depuis des mois, et sans lui il attendrait le prochain commit. Les
vulnérabilités, le code `unsound` et les versions retirées de crates.io
arrêtent la CI ; les crates non maintenues sont signalées sans bloquer.

**Qualité** (`qualite.yml`) — couverture mesurée par `cargo llvm-cov`, puis
analyse SonarQube Cloud. Deux seuils, qui ne disent pas la même chose :

| | Portée | Valeur | Où |
|---|---|---|---|
| Cliquet | tout le dépôt | 42 % des lignes | `SEUIL_LIGNES` dans `qualite.yml` |
| Porte de qualité | code nouveau d'une PR | 80 % | Sonar, `sonar.qualitygate.wait` |

Le premier interdit de redescendre, le second exige 80 % de ce qu'on écrit
désormais. Un seuil global à 80 % aujourd'hui rendrait `main` rouge sans rien
apprendre : la dette se résorbe en la traversant. Il se remonte à la main, à
mesure que le chiffre monte.

Reproduire la mesure :

```bash
cargo llvm-cov --workspace --locked --summary-only
```

L'analyse Sonar demande un projet créé sur [SonarCloud](https://sonarcloud.io)
et un secret `SONAR_TOKEN` dans les secrets Actions ; la marche à suivre exacte
est en tête de `sonar-project.properties`. Tant que le secret manque, le
workflow mesure la couverture, applique le cliquet et passe l'analyse — une CI
rouge faute de compte n'apprendrait rien à personne.
