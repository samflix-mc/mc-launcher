# Packs, manifeste et verrou

[← README](../README.md)

```bash
mc-pack install                     # le pack publié
mc-pack verify [source] [--deep]
mc-pack launch [source] --pseudo Sam

# faire bouger le pack : le manifeste vit dans mc-content
mc-pack lock ../mc-content/launcher/samflix.json
```

## D'où vient la liste des mods

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

## Un chemin ou une URL

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

## Le manifeste

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

## Le verrou

Le verrou est écrit à côté du manifeste et se versionne avec lui, dans
mc-content. Le manifeste dit ce qu'on veut, le verrou dit ce qu'on a eu : la
version retenue quand aucune n'était imposée, et **les dépendances ajoutées
d'elles-mêmes**, chacune avec la raison de sa présence. `install --locked` le
rejoue à l'identique des mois plus tard.

Chaque entrée porte son URL de téléchargement, ce qui rend le verrou lisible
par autre chose que le launcher : la CI de mc-content vérifie que chacune
répond, et un serveur peut installer le jar sans rien savoir de Modrinth.

Rejouer un verrou ne le réécrit pas. Le régénérer effacerait la colonne
`reason` — tout y deviendrait « demandé par le manifeste », puisque c'est le
verrou lui-même qui a dicté les demandes — et on perdrait la seule trace de ce
qui n'avait jamais été demandé.

## Le verrou a la forme du manifeste

Les deux fichiers vivent côte à côte, se lisent l'un après l'autre et se
comparent du regard. Qu'ils nomment la même chose autrement coûte à chaque
lecture : le nom du pack s'appelait `name` dans l'un et `pack` dans l'autre, la
source d'un mod `source` ici et `origin` là.

Le verrou reprend donc le vocabulaire du manifeste, et **tout ce qu'il
déclare** — nom, version, minecraft, chargeur, java, serveurs. Un verrou seul
suffit alors à installer *et* à savoir où se connecter : un script de serveur ou
la CI de mc-content n'ont plus besoin des deux fichiers.

Ce qu'il ajoute, et qui justifie son existence :

| | |
|---|---|
| `generated` | quand il a été écrit |
| `file`, `version`, `channel` | le build exact retenu, et son canal |
| `project`, `file_name`, `url` | de quoi retrouver et télécharger le jar sans rien résoudre |
| `sha1`, `sha512`, `size` | de quoi vérifier ce qu'on a reçu |
| `reason` | pourquoi ce mod est là — demandé, ou tiré par un autre |
| `provides` | les `modId` qu'il fournit, jars embarqués compris |
| `unresolved` | ce que personne n'a su fournir |

Les anciens noms restent **acceptés en lecture** : un verrou publié avant ce
changement se lit sans être réécrit, et le champ `channel` qui lui manque vaut
`release` — supposer une préversion sur un pack que personne n'a touché ferait
apparaître des avertissements infondés.

