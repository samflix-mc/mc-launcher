# Résolution des mods

[← README](../README.md)

Deux sources, essayées dans cet ordre :

| ordre | source | clé | empreinte | remarque |
|---|---|---|---|---|
| 1 | Modrinth | non | SHA-1 + SHA-512 | donne aussi la répartition client/serveur |
| 2 | CurseForge, API publique du site | non | aucune | voir plus bas |

Modrinth passe en premier parce que son API est ouverte — rien à distribuer
avec le binaire — et qu'elle publie les empreintes.

```bash
cargo run -p mc-mods --example resoudre -- --detail jei jade
cargo run -p mc-mods --example resoudre -- --curseforge jade   # force la source 2
```

## Pourquoi plus de clé CurseForge

La Core API de CurseForge demande une clé d'inscription, nominative : un
launcher ne peut pas l'embarquer — elle serait extraite du binaire et révoquée
— et l'exiger de chaque joueur revient à lui demander de créer un compte
développeur pour installer un modpack. Le launcher ne l'interroge plus.

Ce que ce choix coûte, et il faut le savoir :

- **pas d'empreinte publiée.** Le SHA-1 est calculé au premier téléchargement
  et figé dans le verrou ; les installations suivantes sont vérifiées
  normalement, seule la toute première ne l'est pas. En attendant, la taille
  annoncée sert de garde-fou ;
- **plus de `allowModDistribution`.** Ce drapeau, par lequel un auteur refuse
  d'être téléchargé automatiquement par un launcher tiers, n'est servi que par
  la Core API. Il n'est donc plus lisible. Le téléchargement continue de passer
  par la route du site et **jamais** par une URL de CDN reconstruite — c'est
  cette reconstruction qui contournerait activement un refus — mais le refus
  lui-même n'est plus connu. Un auteur qui nous le signalerait doit être retiré
  du pack à la main ;
- **recherche par mot-clé fermée.** Seul un slug exact, tel qu'il apparaît dans
  l'adresse de la page du mod, permet de retrouver un projet. C'est la cause la
  plus fréquente d'un « introuvable » ;
- **cinquante fichiers visibles.** La pagination est ignorée par le serveur.
  Un mod ayant publié plus de cinquante fichiers depuis sa dernière version
  compatible sort de la fenêtre — le cas est détecté et signalé, avec la
  marche à suivre, plutôt que rendu comme « introuvable » ;
- **rien n'est contractuel.** Ces routes servent le site, ne sont pas
  documentées, et cfwidget est un service tiers bénévole.

## Ce que sert l'API publique du site

`www.curseforge.com/api/v1/mods/{id}/files` donne la liste des fichiers avec
leur version et leur chargeur, `/files/{fid}/download` sert le jar, et
`api.cfwidget.com` fournit la seule chose que ces routes refusent : la
correspondance entre un slug et un identifiant de projet.

Vérifié : un jar récupéré par ce chemin est bit-pour-bit identique à celui que
publie Modrinth pour la même version.

**Toutes ces routes enveloppent leur réponse dans `data`**, l'objet isolé comme
la liste. Lire un objet sans son enveloppe donne une désérialisation qui
échoue — et, si l'échec est avalé, un build épinglé bien présent qu'on déclare
introuvable. C'est arrivé.

## Qui l'emporte quand deux branches réclament le même mod

Deux critères, et **l'épinglage passe avant l'origine** :

| | épinglée | ouverte |
|---|---|---|
| manifeste | 12 | 4 |
| dépendance déclarée | 10 | 2 |
| exigence lue dans un jar | 8 | 0 |

Le manifeste a longtemps primé en toutes circonstances. C'était un angle mort :
**une demande sans version n'exprime aucune préférence de version**. Écrire
`{"slug": "sodium"}` dit « je veux ce mod », pas « je veux sa dernière version
quoi qu'il en coûte ».

Le cas qui l'a montré : un pack demandait `sodium` sans version, Iris déclarait
une dépendance vers un build précis de Sodium — ses mixins de compatibilité
visent des classes qui changent de nom d'une version à l'autre. L'ancienne
règle donnait la dernière version à Sodium, les mixins s'appliquaient dans le
vide, et Minecraft tombait à la première connexion sur
`ClassNotFoundException: SodiumGameOptions$PerformanceSettings`.

Le manifeste reste souverain **dès qu'il dit quelque chose** : une demande qu'il
épingle bat tout le reste. C'est la règle de cargo et de npm — une contrainte
stricte l'emporte sur « n'importe quelle version ».

En contrepartie, un mod qui épingle une vieille bibliothèque partagée peut
figer le pack dessus. La parade est la même : épingler dans le manifeste ce
qu'on veut imposer.

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
