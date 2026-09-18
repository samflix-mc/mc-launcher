# L'interface

[← README](../README.md)

L'application : une fenêtre **Tauri 2** dont le front est en **Angular 22**
sans zone, habillé par le design system **Helm**.

Il n'y a **ni Tailwind ni daisyUI**, et ce n'est pas un allègement : le design
system est complet — son propre socle, ses jetons, et six cent vingt lignes de
composants. Faire cohabiter deux systèmes coûtait deux fois, une fois en octets
et une fois en incohérences. Il y avait en plus un piège de cascade :
`@import "tailwindcss"` range ses utilitaires dans une couche, et du CSS SANS
couche l'emporte sur toute couche — un utilitaire posé sur un élément décrit par
le design system aurait été ignoré **en silence**.

```bash
pnpm --dir web install                 # une fois

cd crates/mc-app
cargo tauri dev                        # serveur Angular, puis la fenêtre
cargo tauri build                      # binaire + paquets dans target/release
cargo tauri build --debug --no-bundle  # le même, avec l'inspecteur
```

## Où lancer la commande, et pourquoi cela se redécouvre

La CLI Tauri cherche `tauri.conf.json` en descendant de trois niveaux depuis le
répertoire courant. **Deux positions fonctionnent — la racine du dépôt et
`crates/mc-app` — et `web/` non**, alors que c'est le répertoire où vit le
front et donc le premier qu'on essaie.

Depuis `web/`, il faut donc nommer l'application :

```bash
TAURI_APP_PATH=$PWD/../crates/mc-app pnpm exec tauri build --no-bundle
```

**En chemin absolu.** La variable est canonicalisée, et un chemin relatif qui
ne résout pas fait retomber la CLI **en silence** sur son heuristique : elle
trouve alors une autre configuration, ou pas de configuration, sans rien dire.
C'est ce que fait la CI, qui pose `TAURI_APP_PATH` depuis `github.workspace`.

La CLI est **épinglée dans `web/package.json`** — c'est elle que la CI invoque,
et `cargo tauri` sur le poste doit donc être la même version.

`beforeBuildCommand` vaut `pnpm --dir ../web build`, et c'est correct malgré
l'apparence : le hook s'exécute dans le *répertoire front résolu*, pas dans le
répertoire de l'application. La forme relative est la seule qui marche depuis
les deux positions valides. **Ne pas la « corriger ».**

## Ce que produit le build

Le binaire `helm` et, sous `bundle/`, un `.deb`, un `.rpm` et une
`.AppImage`. Les paquets déclarent `libwebkit2gtk-4.1` et `libgtk-3` et posent
le `.desktop` et les icônes ; l'AppImage les embarque.

Rien à poser dans l'environnement : sous NVIDIA, WebKitGTK rendait une fenêtre
blanche tant qu'on ne lui passait pas `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Le
programme le fait pour lui-même au tout début de `run()`, et seulement si le
module noyau `nvidia` est chargé — voir `crates/mc-app/src/webkit.rs`. Poser la
variable à la main reste possible et l'emporte, dans les deux sens.

`helm --diagnostic` répond sans ouvrir de fenêtre : nom, version,
environnement et sa provenance, DMA-BUF, les quatre emplacements, le
répertoire des journaux. C'est ce que la CI oppose au tag.

## Disposition

```
crates/            tout le Rust, un seul workspace
├── mc-auth/       Microsoft → Xbox → XSTS → Minecraft, et le trousseau
├── mc-chemins/    l'unique endroit qui décide où le launcher range ses affaires
├── mc-dl/         téléchargement vérifié, et l'observation de sa progression
├── mc-java/       détection et installation du runtime
├── mc-instance/   Minecraft, NeoForge, la ligne de commande du jeu
├── mc-mods/       résolution et déploiement des mods
├── mc-nouvelles/  le fil de news : contrat JSON, markdown vers arbre typé
├── mc-pack/       l'orchestration : install, comparaison, jeu, verrou — et la CLI
├── mc-reglages/   les préférences du joueur, leurs bornes, leur persistance
├── mc-log/        journalisation, censure, incidents
├── mc-essais/     un serveur HTTP local, pour les suites
└── mc-app/        l'application Tauri : tauri.conf.json, icônes, src/

web/               tout le front
├── src/app/       les pages, la coque, et `noyau/` — les services
├── src/design/    le design system, deux COPIES : tokens.css et helm.css
├── src/styles.css les polices, les trois imports, et le socle du launcher
├── public/        les assets servis tels quels, dont `splash.html` et les fonds
└── dist/          la sortie de build (ignorée)

target/            la sortie de Cargo (ignorée)
docs/              ce fichier et ses voisins
```

`mc-app` est membre du workspace comme les autres, à une exception près : il ne
figure **pas** dans `default-members`. C'est ce qui l'écarte du `cargo build` nu
du workflow de publication, et du périmètre de `cargo mutants` — muter une
enveloppe demanderait à un test de vérifier une délégation, ce que seul un essai
avec serveur d'affichage pourrait faire. `clippy` et `llvm-cov` le reprennent
par `--workspace`, explicitement.

Ses chemins remontent d'un cran — `frontendDist` vaut
`../../web/dist/launcher/browser` — parce que Tauri les résout relativement à
`tauri.conf.json`.

## Trois fenêtres, et pourquoi

La fenêtre principale charge Angular : un module à analyser, compiler et
exécuter avant le premier pixel. Pendant ce temps, une WebView peint sa couleur
de fond et rien d'autre. `backgroundColor` a supprimé le blanc de cette attente,
pas l'attente.

Une amorce écrite dans `index.html` n'y change rien non plus : elle appartient
au même document et ne s'affiche donc pas plus tôt que lui. La seule façon de
montrer quelque chose **pendant** ce temps est une seconde fenêtre, avec son
propre document.

| | `splash` | `main` |
|---|---|---|
| document | `public/splash.html`, HTML pur, aucun script | Angular |
| à l'ouverture | visible, centrée, au-dessus, hors barre des tâches | **cachée** |
| décorations | aucune | aucune |

Le front décide que c'est fini : lui seul sait quand il a rendu. Il appelle
`front_pret` après son premier rendu, plus 250 ms. Rust ne peut pas le
deviner — `RuntimeRunEvent::Ready` dit que la fenêtre *existe*, pas que son
contenu est peint.

Deux bornes encadrent ce passage, et elles ne servent pas à la même chose :

- un **plancher de 2 s** — en dessous, l'écran de démarrage passe trop vite
  pour être lu, et ressemble à un défaut d'affichage plutôt qu'à une intention.
  Mesuré sur un poste ordinaire, le front signale au bout d'environ 600 ms ;
- un **plafond de 10 s** — si le front n'appelle jamais, par exemple sur une
  erreur JavaScript, la principale resterait cachée pour toujours derrière un
  écran sans le moindre bouton.

Les deux se mesurent depuis l'instant où les fenêtres existent, et c'est
pourquoi elles vivent en Rust : le front ne sait pas quand l'écran de démarrage
est apparu, il ne sait que quand lui-même a fini.

Puis l'amorce Angular prend le relais derrière, avec son propre plancher de
900 ms, le temps que la poignée de main réseau se fasse.

### La troisième : la connexion

Elle fait 440 × 520, comme le design system la pose, et elle est **créée à la
demande** — pas déclarée dans `tauri.conf.json`, sinon elle s'ouvrirait à chaque
démarrage, y compris pour qui a déjà une session.

```text
  splash ──┬── session trouvée ──→ main
           └── aucune session  ──→ connexion ──→ main
```

Elle charge la MÊME application Angular, à la route `/connexion`. Le front s'y
reconnaît par l'étiquette de sa fenêtre — `getCurrentWindow().label` — et se
dessine autrement : feuille dépolie pleine surface, barre de titre sans bouton
d'agrandissement ni cloche, pas de navigation, pas de bouton de jeu, pas de
badge joueur.

Quatre choses valent d'être connues, parce qu'elles ne se déduisent pas :

- **La fenêtre principale ne montre jamais la connexion.** Sa garde l'y envoie
  comme partout ailleurs, mais elle demande alors l'ouverture de la fenêtre
  dédiée et s'efface. Elle reste sur cette route pendant qu'elle est cachée, et
  la quitte sur le signal `session-ouverte`.
- **Cacher le bouton d'agrandissement ne suffit pas.** `--dialog` le retire du
  gabarit ; c'est `maximizable: false` à la construction qui empêche le
  double-clic sur la barre d'agrandir quand même.
- **Fermer la connexion quitte le launcher.** La principale est cachée, il n'y a
  rien derrière, et un processus qui survit à sa dernière fenêtre visible ne se
  retrouve que dans le gestionnaire de tâches. La garde tombe dès que la session
  est ouverte.
- **`capabilities/default.json` nomme les DEUX fenêtres.** Une fenêtre absente de
  la liste `windows` s'ouvre normalement et voit chacun de ses appels refusés —
  ses boutons de barre de titre ne font rien, et rien ne le dit en
  développement.

## La barre de titre

`decorations: false` : la barre est dessinée par l'application. Sans cela, on
en avait **deux** — celle du système et la nôtre.

Ce qui reste au système, et qu'il ne faut pas réécrire : **le redimensionnement
par les bords**. Le runtime branche lui-même un gestionnaire sur la WebView, sur
une bande de 5 px multipliée par le facteur d'échelle. Y ajouter des zones DOM
ferait double emploi.

Ce qu'il faut écrire, en revanche, c'est la **contrainte de gabarit** : aucune
cible de clic ne commence à moins de **8 px** d'un bord. Sans ce retrait, tirer
la fenêtre par le haut la redimensionne au lieu de la déplacer — le gestionnaire
GTK s'exécute avant que WebKit ne dispatche le `mousedown`. Et comme la garde
ne s'applique pas quand la fenêtre est maximisée, **le même bouton se comporte
différemment selon l'état de la fenêtre**. Aucun test jsdom ne le verra.

Pas de `pointer-events: none` sur les enfants de la zone de déplacement : le
script de Tauri écarte déjà de lui-même les `a`, `button`, `input`, `select`,
`textarea`, `label`, `summary`, tout `[contenteditable]`, tout `[tabindex]`
non négatif et tout rôle interactif. Le neutraliser coûterait le survol et le
curseur sans rien acheter.

La capacité déclare nommément `minimize`, `toggle-maximize`, `close`,
`start-dragging` et `is-maximized` : `core:window:default` ne les contient pas.

**Ce qui manquait n'était pas le geste mais le curseur.** Le gestionnaire GTK
redimensionne bien, mais rien dans le document ne demandait `ns-resize` ou
`ew-resize`, et WebKit dessine le sien par-dessus celui de GTK. Huit zones de
huit pixels — `.hm-bords` — portent donc le curseur et **aucun gestionnaire**.
Elles disparaissent quand la fenêtre est maximisée : la garde `!is_maximized()`
du runtime désarme le geste, et promettre ce que rien n'exécute est pire que de
ne rien promettre.

La barre porte, de gauche à droite : le nom du launcher en face pixel, un
séparateur, le nom du modpack, puis la cloche des notifications et les trois
contrôles. Les contrôles font quarante-six pixels, la largeur des boutons de
légende de Windows ; la cloche en fait trente-deux, parce que ce n'est pas un
bouton de légende.

## Les quatre pages

| Route | Ce qu'on y fait | Barre du bas |
|---|---|---|
| `/connexion` | la session Microsoft, dans SA PROPRE FENÊTRE, et l'état « ce compte ne possède pas le jeu » | aucune, et pas de coque non plus |
| `/spawn` | la nouvelle épinglée, l'état du pack, la cinématique pendant le travail | le bouton et le badge joueur |
| `/nouvelles` | la tuile vedette et la grille | le badge joueur seul |
| `/configuration` | Apparence, Fenêtre du jeu, Vidéo, Java, Avancé | aucune |

**Le rail de la Configuration ne met rien dans l'URL.** Ses entrées étaient des
ancres `#reglages-…` et ne fonctionnaient pas : du temps de
`withHashLocation()`, le dièse appartenait au routeur. Ce sont des boutons qui
font défiler — ce qui reste juste depuis que le fragment a disparu, parce que
faire défiler dans une page n'est pas naviguer, et qu'une ancre y laisserait une
entrée d'historique que le bouton « précédent » relirait comme un changement de
page.

**Les réglages se règlent en deux temps.** Les contrôles lisent un brouillon
local ; `(input)` ne met à jour que lui, `(change)` — le relâchement — écrit.
La première version enregistrait sur `(input)` : tirer un curseur partait en
trente à cinquante allers-retours par seconde, les réponses revenaient dans le
désordre, et le nombre affiché restait figé pendant qu'on tirait.

Ce que porte la barre du bas est une **donnée de route** — `data: { bas }` — et
non un test sur l'URL : une chaîne comparée à `'/spawn'` se casse le jour où une
route gagne un paramètre, et le symptôme est une barre du bas vide que rien
n'explique.

**Tant que la session n'est pas jouable, il n'y a pas de coque** : ni pilule de
navigation, ni bouton de jeu, ni badge joueur. Montrer un menu et un bouton « se
déconnecter » à quelqu'un qui n'est pas connecté était le premier reproche de la
recette.

Toutes en `loadComponent`, et l'historique en chemins RÉELS — `/spawn`, pas
`#/spawn`.

Le fragment a été employé un temps, par prudence : ne rien devoir au repli SPA
du protocole d'actifs de Tauri, qu'on tenait pour un détail d'implémentation.
Il coûtait plus qu'il ne protégeait — le dièse appartient au routeur dès qu'il
est en jeu, et aucune ancre ne peut plus servir à autre chose dans la page.

Le repli est bien là, et vérifié dans les sources de la version épinglée :
`tauri-2.11.5/src/manager/mod.rs` enchaîne quatre tentatives pour un actif
introuvable — `<chemin>.html`, `<chemin>/index.html`, puis `index.html`. Et
`ng serve` fait la même chose. Les deux environnements où ce launcher tourne
servent donc `index.html` pour une route inconnue.

Si la fenêtre s'ouvrait blanche un jour, deux choses à vérifier dans cet ordre :
cette quatrième tentative, et `<base href="/">` dans `index.html`, sans lequel
les actifs se résoudraient contre `/spawn/`. Le greffon de navigation, lui, n'y
est pour rien : son prédicat porte sur l'ORIGINE, jamais sur le chemin.

**Les deux gardes lisent le même prédicat**, `jouable()`. Deux prédicats
différents feraient rebondir sans fin un compte connecté mais sans licence
entre `/connexion` et `/spawn` ; ici, ce cas est un **état** de la page
Connexion.

Pas de `withDisabledInitialNavigation()` : une garde peut rendre une promesse,
le routeur l'attend, et aucune URL n'est validée tant qu'elle pend. Il n'y a
donc pas de saut à empêcher. Le drapeau, en revanche, exige un
`router.initialNavigation()` explicite dont l'oubli donne une fenêtre bloquée
sur son écran de démarrage, **sans une erreur en console**.

## Un seul bouton, deux gestes

Le launcher récupère le verrou publié — quelques kilooctets — et compare son
empreinte, sur la forme canonique et non sur les octets reçus, à celle du verrou
posé. Il sait donc, avant de proposer quoi que ce soit, s'il y a quelque chose à
rattraper.

**Ce qu'il en fait a changé à la recette.** Le bouton a d'abord enchaîné les
deux : « Mettre à jour et jouer » posait la mise à jour puis lançait Minecraft.
Sam l'a repris là-dessus — « ça lance le jeu alors qu'on voulait juste installer
le modpack » — et le motif est net : poser huit cents mégaoctets et jouer sont
deux intentions, et la seconde ne se déduit pas de la première.

Dès qu'il y a quelque chose à poser, **le bouton pose et s'arrête** ; il devient
alors « Jouer », et c'est un second clic. Ce que le geste unique avait résolu
n'est pas perdu pour autant : la vérification reste en tête des deux chemins,
elle coûte toujours quelques dizaines de kilooctets, et personne n'attend un
téléchargement pour jouer à un pack déjà à jour.

La règle vit en **un seul endroit** — `mc_pack::comparaison::a_poser` — que
`Action` expose au front et que `jeu::doit_rattraper` suit. Deux copies
diraient un jour deux choses différentes, et le bouton proposerait de jouer à un
pack que l'installation vient de décider de reposer.

Il vit dans la **coque** et non dans Spawn : ce n'est pas une décision de la
page, c'est l'état du disque et celui de la session qui le pilotent. Le design
system le place au centre de la barre du bas.

| Ce que le disque dit | Ce que le bouton dit |
|---|---|
| on ne sait pas encore | « Vérification… », et surtout pas « Installer » |
| rien d'installé | **Installer** |
| installé et conforme | **Jouer** |
| installé, verrou différent | **Mettre à jour** — et rien d'autre |
| génération changée | **Réinstaller** — et rien d'autre |
| une opération en cours | le remplissage et le pourcentage, plus le débit quand quelque chose descend |
| le jeu tourne | « En jeu », inerte |
| hors ligne et rien d'installé | inerte, avec la raison |

**Les deux lignes au-dessus du bouton ne sont pas décoratives.** La première dit
POURQUOI le bouton est ce qu'il est ; sans elle, l'écran demande de deviner — ce
qui était exactement le reproche fait à l'ancienne page Spawn. Pendant le
travail, elle nomme l'étape et son rang.

La seconde n'existe que pendant le travail, et c'est l'autre retour de recette :
« on sait à peu près à quelle étape on est, mais pas ce qu'on télécharge, ni à
quelle vitesse ». Elle porte le fichier en cours, le compte de fichiers, les
octets, le débit et le temps restant — toutes des valeurs que `Suivi` émettait
déjà cinq fois par seconde et que personne n'affichait.

Le pourcentage, lui, **survit aux creux entre deux lots**. `actif` retombe à
faux pendant la résolution des mods, l'inspection des jars et l'installateur
NeoForge : le bouton n'affichait alors plus aucun chiffre, ce qui se lit comme
un blocage. Il suit la progression GLOBALE, bornée par la phase atteinte — entre
deux lots elle stagne, ce qui est la vérité, là où la progression d'un lot
sauterait à cent pour cent.

Entre l'affichage de la page et la réponse de `etat_du_pack()`, le bouton dit
qu'il regarde. Une valeur par défaut « Installer » produirait un clignotement
INSTALLER → JOUER à chaque démarrage, sur le seul élément de l'écran qui compte.

Sans réseau, la comparaison ne rend pas une erreur : l'écart est *inconnu*, le
fil est marqué hors ligne, et le bouton dit JOUER si le disque porte un pack
cohérent.

**Le CLI garde ses deux commandes** : `mc-pack install` et `mc-pack launch`
sont des outils d'outilleur, et l'un ne doit pas déclencher l'autre.

## La cinématique

Onze phases, chacune portant son état : faite, en cours, à venir. C'est ce qui
distingue « on en est à la moitié » de « il se passe quelque chose ».

**Elle ne s'affiche nulle part**, et c'est une décision de recette. Le bouton
porte déjà la progression globale, son pourcentage et son débit ; la phrase
au-dessus de lui NOMME l'étape en cours. Une liste des onze phases disait donc
une troisième fois ce que deux éléments disaient déjà, au prix d'un panneau qui
apparaissait et disparaissait sous le regard, à l'endroit même où l'on suit
l'avancement.

Le modèle reste — `Pack.etapes` annote chaque phase de son état, et
`etapeCourante` donne « 5 sur 9 · Fichiers du jeu ». C'est ce dernier que la
barre du bas affiche. Les libellés ont été réécrits au passage : « Pack » et
« Verrou » ne voulaient rien dire pour un joueur.

| | |
|---|---|
| Compte Microsoft | session reprise du trousseau, ou ouverte par code d'appareil |
| Licence Minecraft | `Auth::owns_game` |
| Lecture du pack | manifeste, verrou, et la purge si la génération a changé |
| Version du chargeur | `latest` résolu tout de suite, pour que le verrou porte un numéro |
| Fichiers du jeu | client, bibliothèques, assets |
| Java | détecté, ou Temurin posé à la majeure **exacte** du verrou |
| Installation de NeoForge | l'installateur officiel, dans le Java ci-dessus |
| Mods | résolus, téléchargés, répartis client/serveur |
| Finalisation | le verrou, écrit en dernier : il décrit ce qui a réellement été fait |
| Prêt à jouer | le bouton s'allume |
| Jeu lancé | |

Les sept du milieu sont `Etape::TOUTES` de `mc-pack`, dans l'ordre : chacune
dépend de la précédente. Voir [packs.md](packs.md).

**Un seul écart avec la ligne de commande, délibéré** : la licence est vérifiée
*avant* d'installer. `mc-pack` ne la regarde jamais, et faire attendre huit
cents mégaoctets à un joueur pour lui apprendre ensuite que son compte n'a pas
le jeu serait une faute.

La cinématique **ne recule pas**. L'étape Java n'est émise que si un runtime
doit réellement être posé : sur un rattrapage, le chemin est déjà sur « prêt »
quand la vérification commence, et rallumer une étape passée se lirait comme
une régression.

### Le débit et le temps restant

`mc-dl` lit chaque corps morceau par morceau et annonce ce qui arrive — des
dizaines de milliers d'événements par minute. Les passer tels quels au pont
noierait la fenêtre : ils sont accumulés dans des compteurs atomiques, et
l'état complet part cinq fois par seconde. Ce qui s'affiche est une photo, pas
un flux.

Le total vient des lots que les crates annoncent avant de commencer : l'index
des assets publie la taille de chaque objet, le descripteur de version celle de
chaque bibliothèque, les métadonnées de projet celle de chaque jar. Il est donc
exact chez Mojang, et **un plancher** pour les mods de CurseForge sans clé, qui
n'en publient pas — la barre accélère alors à la fin, et le temps restant
disparaît plutôt que d'afficher zéro.

Deux sources font avancer la barre sans jamais se compter deux fois : les
octets descendus du réseau, et le poids des fichiers déjà conformes sur le
disque. Sans les seconds, une réinstallation resterait à zéro de bout en bout
alors que tout est déjà là.

La barre elle-même **n'est pas un `<progress>`**, et pour une raison qui a coûté
un essai : la feuille d'agent de WebKitGTK ne porte aucune règle
`:indeterminate`, et ce qui anime une barre native relève précisément du rendu
que `appearance: none` éteint — or `appearance: none` est obligatoire pour la
direction artistique. On aurait donc eu un bloc **figé à 50 %**, c'est-à-dire le
mensonge exact qu'une jauge existe pour ne pas dire.

Ce sont les barres du design system : `.hm-progress__fill` pour une valeur
connue, `--indeterminate` pour une valeur qui ne l'est pas, et `.hm-play__fill`
à l'intérieur même du bouton pendant l'installation. Le pourcentage passe par
`[style.--p]`, une propriété personnalisée que le CSS consomme.

## Le design system, et ce qu'il décide

Tout ce qui se voit vient de `docs/helm-design-system.zip`, recopié dans
`web/src/design/` en **deux fichiers qu'on remplace en entier** :

| Fichier | Ce qu'il porte |
|---|---|
| `tokens.css` | les couleurs, les échelles, les rayons, les ombres, les familles |
| `helm.css` | les composants, préfixés `hm-` |

Deux écarts avec le zip, et ils sont écrits dans les en-têtes des copies :

1. **L'`@import` vers Google Fonts a été retiré.** `default-src 'self'`
   refuserait la requête, et une fenêtre hors ligne — ce qui arrive à un
   launcher — se retrouverait sans police. Les trois familles (Manrope,
   Pixelify Sans, JetBrains Mono) sont servies par le paquet, depuis
   `@fontsource`, en sous-ensembles `latin` et `latin-ext`.
2. **Un bloc du launcher est ajouté à la fin de `helm.css`**, sous une barre qui
   le dit : la fenêtre du système n'est pas un rectangle dans une page de
   démonstration, et les pages d'aperçu posent leur grille en `style=`, ce qu'un
   gabarit Angular n'a pas à recopier.

Le reste est verbatim. Une correction faite ailleurs que dans le bloc ajouté
disparaîtrait à la copie suivante sans que rien ne s'en aperçoive.

## La scène, le voile, le verre

L'écran est une **scène** — `.hm-stage` : l'image du serveur, puis le dégradé de
lisibilité du design system (le fond à 52 % en haut, 36 % au milieu, 80 % en
bas). Par-dessus flottent la barre de titre, la pilule de navigation, les
panneaux en verre et la barre du bas. **Le milieu de la page est laissé vide à
dessein** : c'est par là que l'image passe, et c'est le seul endroit de l'écran
où elle se voit.

`fond` est un **identifiant énuméré**, jamais un chemin : sinon le front
posséderait la liste et Rust persisterait une valeur qu'il ne sait pas valider.
Le service de réglages pose `data-fond` et `--voile` sur `<html>` — un style de
composant ne peut pas déclarer sur `:root`, que l'encapsulation émulée
réécrirait en `:root[_ngcontent-…]`.

### Le voile ne tient plus le contraste, et c'est le point important

Il l'a tenu, à 0,35 puis à 0,44, sur une mesure juste : « à partir de quelle
opacité le texte tient-il 4,5:1 sur l'image la plus claire concevable ? »
C'était la **question** qui ne l'était plus. Elle supposait que le contraste se
règle en assombrissant l'image ; le design system y répond autrement, et
mieux — « sur une image claire, montez la base du verre plutôt que d'assombrir
le texte ». Ce qu'on épaissit est le **panneau**, pas l'image.

La différence n'est pas théorique. Un voile à 0,44 s'applique partout, y compris
là où il n'y a aucun texte : le launcher affichait un rectangle noir à l'endroit
même que toute sa direction artistique existe pour montrer.

La garantie a donc changé de couche, et elle tient en trois endroits :

1. `.hm-stage__art::after`, le dégradé du design system — non réglable ;
2. `--glass-epaisseur` dans `styles.css`, qui **épaissit le verre quand le voile
   s'amincit** : de 46 % du fond à 72 %. À voile nul et sur une image blanche,
   `ink-3` — la teinte la plus claire que le design system autorise — tient
   encore 4,5:1 sur un panneau ;
3. les ombres portées des textes sans panneau sous eux : les titres de tuile et
   l'indication du bouton de jeu.

`VOILE_PLANCHER` vaut donc **zéro**, et le curseur va de « image nette » à
« image effacée ». À rouvrir si l'on retire la compensation du point 2 — et dans
ce cas, c'est cette mesure-là qu'il faut refaire, pas celle d'avant.

### `-webkit-backdrop-filter`, et le piège qui se referme des semaines plus tard

La WebKitGTK installée n'expose que la forme **préfixée**. Angular ne passe pas
par autoprefixer : c'est esbuild qui préfixe, et il ne le fait qu'en dessous de
Safari 18. D'où la ligne :

```json
"browserslist": ["safari 17.4", "chrome 120", "edge 120"]
```

Son motif n'est pas la syntaxe moderne — le launcher ne tourne que dans
WebKitGTK, WKWebView et WebView2 — c'est de **forcer le préfixe**. Sans elle, le
verre dépoli est un aplat dans toute la fenêtre.

Le design system écrit lui-même les deux formes, ce qui rend la ligne moins
critique qu'elle ne l'était : c'est une ceinture pour le CSS que nous écrivons —
les styles de composant — et non plus pour des utilitaires produits par un
outil. `pnpm --dir web verifier:prefixes` regarde le CSS **compilé**, sous
`web/dist/`, et pas les sources : un préfixe écrit à la main dans un essai le
ferait passer sans que le reste suive.

## Le CSP, et ce qui est réellement servi

Les deux modes ne servent pas la même page. En `tauri dev`, c'est le serveur
Angular qui sert, **sans CSP** ; en production, c'est Tauri, avec celui de
`tauri.conf.json`, réécrit.

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' data: https://mc-heads.net;
connect-src 'self' ipc: http://ipc.localhost
```

`style-src` est desserré **exprès** : c'est ce qui achète les `styleUrl` de
composant, sans lesquels tout le style redescendrait dans un unique fichier
global. `script-src` reste strict, et c'est lui qui protège l'origine
privilégiée où `invoke` est joignable.

`dangerousDisableAssetCspModification: ["style-src"]` accompagne le desserrage,
et sa raison n'est pas l'état d'aujourd'hui : c'est que `index.html` porte du
balisage — l'amorce statique. Le jour où quelqu'un y poserait une balise
`<style>`, Tauri injecterait un nonce sur `style-src`, et **un nonce annule
`'unsafe-inline'`** : tous les styles de composant tomberaient, **en build
empaqueté seulement**. Le réglage est une ceinture contre une évolution prévue.

Ce que la lecture du code prédisait a été **mesuré**, le 18 septembre 2026, sur
un `tauri build --debug` :

```
style-src 'self' 'unsafe-inline';
img-src 'self' data: https://mc-heads.net;
default-src 'self';
connect-src 'self' ipc: http://ipc.localhost;
script-src 'self' 'sha256-…' ×8
```

Et cela corrige une prémisse : Tauri ne pose **pas** de nonce, il pose les
**empreintes** des scripts qu'il injecte lui-même. `style-src` sort intact. Le
résultat est celui qu'on voulait, mais le raisonnement qui le prédisait parlait
d'un nonce — et un raisonnement juste pour une mauvaise raison se retourne à la
première montée de version.

`inlineCritical` **reste `false`** : l'outil réécrit le `<link>` en `onload=`,
qui relève de `script-src`.

### La condition qui rend le desserrage acceptable

`'unsafe-inline'` sur `style-src` ne tient que parce qu'**aucun balisage ne
vient d'ailleurs que du compilateur Angular**. Un seul `innerHTML` — sur un
titre de news, sur un message d'erreur venu de Rust — et du texte distant
deviendrait du DOM dans une origine privilégiée.

C'est pourquoi le markdown des nouvelles est analysé **en Rust**, vers un arbre
typé que le front rend avec un `@switch`. Ce qui n'est pas reconnu devient du
texte, jamais du balisage : un `<script>` écrit dans un billet ressort comme les
onze caractères qu'il est.

Et c'est pourquoi `pnpm --dir web invariants` existe : il refuse `.innerHTML =`,
`[innerHTML]` et `innerHTML:` dans `web/src`. Un invariant dont on a perdu la
raison finit par être retiré — celui-ci porte la sienne, dans le script.

### La seule origine distante, et ce qu'elle coûte

`img-src` autorise `https://mc-heads.net`, et rien d'autre de distant. C'est la
tête du joueur, rendue en trois dimensions.

Le choix inverse avait été fait d'abord — une pastille locale, les initiales du
pseudo sur une teinte dérivée de l'UUID — au motif qu'envoyer l'UUID d'un joueur
à un tiers pour une image décorative n'en valait pas le prix. Il est revenu sur
la table pour deux raisons. L'UUID n'est pas un secret : **tout serveur où le
joueur se connecte le connaît**, et il figure déjà dans le verrou. Et une tête
de skin n'est pas décorative dans un launcher de serveur moddé — c'est ce qui
dit au joueur *quel compte* est connecté, sur une machine qui en a souvent eu
plusieurs.

Ce qui reste vrai de l'ancien argument est conservé ailleurs : `format.ts` garde
`initiales()`, qui sert partout où l'on ne veut pas d'appel réseau. Et
l'autorisation est nominative — un domaine, pas un joker : un `img-src *` ferait
de chaque champ d'URL du fil de news un moyen de joindre n'importe quel hôte.

### La sonde, et l'inspecteur

L'inspecteur **n'existe pas en release**. Pour l'ouvrir :

```bash
cd crates/mc-app && cargo tauri build --debug --no-bundle
../../target/debug/helm
# puis clic droit dans la fenêtre → Inspecter l'élément
```

Mais l'en-tête ne se lit pas seulement là. En compilation de développement,
`crates/mc-app/src/csp.rs` demande à la fenêtre ce qu'elle a **reçu** et le
journalise avec son verdict :

```
INFO CSP servi à la fenêtre entete="…" styles_de_composant_passent=true
     nonce_sur_style=false scripts_stricts=true
```

Une sonde plutôt qu'un coup d'œil : l'inspecteur demande un geste humain et ne
répond que pour le build qu'on a sous la main ce jour-là. La ligne, elle, change
toute seule le jour où quelqu'un ajoute un `<style>` dans `index.html`. Sa
moitié qui juge est une fonction pure, éprouvée sans serveur d'affichage.

La règle qui s'en déduit reste la même : **une modification de l'interface se
regarde au moins une fois dans un `cargo tauri build`.**

## La recette du build empaqueté

Huit points décident qu'un build empaqueté rend ce que `ng serve` montre, et
**aucun ne se voit en développement** : c'est en production seulement que Tauri
sert la page, réécrit le CSP et applique les `import()` dynamiques.

Ils ne se déroulent plus à la main. `crates/mc-app/src/acceptance.rs` les joue à
chaque lancement d'une compilation de développement, et les résume en une
ligne :

```
INFO recette du build : les huit points passent
     theme="#e6b54a" verre="blur(24px) saturate(1.6)"
     route_montee=true feuilles=1
```

| Point | Ce qui le prouve |
|---|---|
| le CSS est servi par `<link>` | `feuilles` ≥ 1 |
| le verre dépoli rend | `verre` porte une valeur calculée — c'est le test du préfixe `-webkit-` |
| **un fragment paresseux se charge** | `route_montee` — c'est le test de `script-src` face à `import()`, longtemps affirmé sans preuve |
| les jetons du design system sont servis | `theme` porte `--gold` |
| **un style de composant s'applique** | `style_composant` porte le repère d'`app.css` |
| `color-mix()` est résolu par le moteur | une sonde jetable dans le DOM, dont la couleur ressort en `rgb(…)` |
| aucune violation de CSP | un collecteur posé **avant le document**, par un script d'initialisation de greffon |
| le bundle initial tient le budget | lu dans la sortie du build : **355,61 kB** pour 500 kB d'avertissement |

Trois choses valent d'être connues sur cette sonde, parce qu'elles ont chacune
coûté un essai :

- **elle attend la route.** Observée juste après le premier rendu, elle tombe
  pendant l'amorce et conclut que le fragment n'est jamais arrivé — sur un
  build parfaitement sain ;
- **elle n'utilise pas la Resource Timing API.** Le protocole d'actifs de Tauri
  ne la renseigne pas : compter les `.js` chargés rendait zéro ;
- **elle ne cherche pas `data-theme`.** L'attribut est écrit en dur dans
  `index.html` et y serait même si `tokens.css` n'était pas servi : le trouver
  ne prouverait rien. Ce qui se vérifie est qu'un jeton est arrivé.

Le repère de style de composant mérite un mot, parce qu'il a l'air inutile :
`--recette-style-composant` est déclarée dans `web/src/app/app.css` et nulle
part ailleurs. Le point portait avant sur le `backdrop-filter` de la barre de
titre ; il ne le prouve plus, puisque le verre vient maintenant de la feuille
globale du design system, servie par un `<link>`, qui passerait même si tous les
styles de composant étaient rejetés. **Son inutilité est ce qu'il mesure.**

### Sans décorations : ce qu'on perd, constaté

Relevé sur capture le 18 septembre 2026, **KDE Plasma sous Wayland** :

| | |
|---|---|
| Barres de titre | **une seule**, la nôtre — c'était le défaut à corriger |
| Coins | **droits**, pas arrondis |
| Ombre portée | **absente** |

Les deux dernières lignes sont le coût réel de `decorations: false`, et il est
payé : le gestionnaire de fenêtres ne décore plus, donc il n'ombre plus et
n'arrondit plus. Les récupérer demanderait `transparent: true` et un
`border-radius` sur la racine — ce qui échange un défaut cosmétique contre une
fenêtre translucide sous un moteur qui compose déjà mal, et n'a pas été fait.

**GNOME/Wayland n'a pas pu être vérifié** : la machine de développement n'a que
la session Plasma d'installée. Le comportement y est réputé identique — c'est
le même protocole et le même runtime — mais ce n'est pas un constat.

### Ce qui reste à l'œil

La sonde dit que `backdrop-filter` **est calculé**, pas qu'il est **joli**.
Ce qui relève du gestionnaire de fenêtres ou du goût se regarde, et la liste
est courte :

1. tirer la fenêtre par les 8 px du haut : elle doit **se déplacer**, pas se
   redimensionner. Puis la maximiser et recommencer — la garde du runtime ne
   s'applique plus, et c'est là que le comportement diverge ;
2. le verre dépoli sur une image de fond qui **défile** : c'est le seul des
   trois cas de `backdrop-filter` qui décide, les deux autres passent toujours ;
3. l'apparence sous GNOME, tant que personne n'y aura ouvert la fenêtre.

## La navigation, sur liste blanche

Un greffon `on_navigation` refuse toute URL hors de l'origine de
l'application. Sous forme de greffon parce que le hook n'existe pas sur
`tauri::Builder` : seulement sur un constructeur de webview — or la nôtre est
déclarée dans `tauri.conf.json` et n'existe pas encore à ce moment-là — ou de
greffon, dont le magasin est consulté pour toute webview.

La liste doit couvrir plus que `tauri://` : l'origine est `tauri://localhost`
sous Linux et macOS, `http://tauri.localhost` sous Windows,
`http://localhost:1420` en `tauri dev`. **Trop strict, le prédicat rend une
fenêtre blanche sans un mot** — chaque refus se journalise donc en `warn` avec
l'URL. Le prédicat est une fonction pure, éprouvée en sidecar.

Les liens externes des billets ne naviguent pas : ils passent par
`tauri-plugin-opener`, qui les ouvre dans le navigateur du système.

## Ce que l'application fait, et ce qu'elle ne fait pas

Elle n'ajoute **aucune** logique de launcher. `mc-auth` authentifie,
`mc_pack::install` installe, `mc_pack::jeu` compare, rattrape et lance,
`mc-nouvelles` lit le fil, `mc-reglages` garde les préférences, `mc-chemins`
décide des emplacements, `mc-log` journalise. `mc-app` appelle, agrège, et
raconte : c'est tout.

C'est délibéré, et cela s'est vérifié à l'usage. Le lancement vivait dans le
**binaire** `mc-pack` : tant qu'il n'y avait qu'une ligne de commande, cela ne
coûtait rien. Dès que la fenêtre a voulu lancer le jeu, elle n'a pas pu
l'appeler — et a réassemblé la session, le Java du verrou et la ligne de
commande de son côté, en sautant la vérification de cohérence de l'instance.
Deux assemblages pour la même chose, dont un seul correct.

Même histoire pour le trousseau. Il a d'abord été écrit dans `mc-app`, et
`mc_auth::charger()` continuait de lire le fichier : la fenêtre enregistrait à
un endroit que la ligne de commande ne regardait pas. Il vit dans
`crates/mc-auth/src/stockage/`, et les deux voient le même compte.

Même histoire, enfin, pour les chemins : chaque crate dérivait les siens. C'est
`mc-chemins` qui décide, et `crates/mc-app/src/paths.rs` qui pose ceux du
résolveur de Tauri au démarrage.

`mc-log` plutôt que `tauri-plugin-log`, d'ailleurs : le second écrirait les
jetons tels quels. Une fenêtre graphique avale sa sortie standard, donc le
fichier de journal est tout ce qu'un joueur pourra joindre à un rapport — c'est
exactement le moment où il ne faut pas qu'un jeton s'y trouve.

### L'ordre du démarrage, qui se lit mal

```rust
webkit::regler_le_rendu();          // écrit l'environnement, avant le moindre fil
if diagnostic::demande(…) { … }     // répond et sort, sans serveur d'affichage
let application = Builder::…build(…)?;
chemins::poser(&application);       // le résolveur répond déjà
let _journal = mc_log::init(…);     // le fichier s'ouvre au bon endroit
application.run(|_, _| {});         // et c'est lui qui crée les fenêtres
```

Le piège est au milieu : juste après `build()`, `get_webview_window("main")`
rend `None`. Les fenêtres déclarées dans `tauri.conf.json` sont construites à
l'intérieur de `run()`. C'est pour cela que le hook `.setup()` **reste** : y
substituer un appel direct après `build()` ne ferait rien — sans un
avertissement — et la fenêtre garderait le titre figé du fichier de
configuration.

`build()` est enveloppé dans un `match` dont la branche d'erreur écrit dans le
répertoire des journaux avant de paniquer : un poste sans serveur d'affichage
sortirait sinon sur la seule sortie d'erreur d'une application graphique, sans
journal ni Sentry, puisque `mc_log::init` n'a pas encore tourné.

## Où va le jeton

L'état d'authentification porte un jeton de rafraîchissement : il rouvre le
compte sans mot de passe ni second facteur. Il va au **trousseau du système** —
Secret Service, Keychain, Credential Manager — sous le service `samflix-mc`.

Une machine sans trousseau retombe sur un fichier `0600`, avec un `warn` dans le
journal. La lecture essaie les deux, dans cet ordre : une session ouverte par
`mc-auth login` est reprise ici sans rien redemander. Le détail, et pourquoi la
déconnexion vide les deux côtés, sont dans `crates/mc-auth/src/stockage/` et
dans [authentification.md](authentification.md).

## Pourquoi Tauri 2 et pas 3

Au 17 septembre 2026, la 3.x est en **alpha** — pas en bêta —, publiée deux
jours plus tôt, sans guide de migration ni documentation, et son écosystème de
plugins est déjà désynchronisé. Surtout, elle ne change rien à ce qui nous
concerne : sur le runtime `wry`, le rendu Linux reste WebKitGTK, avec le même
défaut DMA-BUF. La seule porte de sortie serait `tauri-runtime-cef`, qui
remplacerait WebKitGTK par Chromium — mais il est alpha et ne démarre pas sous
Linux. On échangerait un contournement connu contre un bug non contournable.

À reconsidérer quand une bêta 3.x paraîtra avec un guide de migration, et en
priorité si le runtime CEF devient fiable.

## Trois familles d'attributs, et une seule est un contrat

`class` n'est lue que par le navigateur — et par le design system, qui les
possède toutes. `data-<état>` est lue par le style **et** par les tests.
`data-test` est lue par les tests seuls.

Un test qui vise une classe se casse au premier changement de mise en forme —
et, pire, il décourage de la changer. C'est le second invariant de
`web/scripts/invariants.mjs`, et il est **actif** : plus aucun
`querySelector('.…')` dans `web/src`.

## Contrôles

```bash
pnpm --dir web test:ci                 # vitest
pnpm --dir web invariants              # innerHTML, sélecteurs de classe
pnpm --dir web format:check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Puis, au moins une fois avant de fusionner une modification de l'interface :

```bash
cd crates/mc-app && cargo tauri build --debug --no-bundle
../../target/debug/helm    # et lire la ligne « CSP servi »
pnpm --dir web verifier:prefixes       # le préfixe, dans le CSS compilé
```
