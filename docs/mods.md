# Résolution des mods

[← README](../README.md)

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

## CurseForge sans clé

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

## Les dépendances qu'aucune API ne déclare

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
