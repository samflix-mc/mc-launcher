# L'interface

[← README](../README.md)

L'application : une fenêtre **Tauri 2** dont le front est en **Angular 22**, sans
framework de style — du CSS et rien d'autre.

```bash
pnpm --dir web install     # une fois
cd crates/mc-app
cargo tauri dev            # démarre le serveur Angular puis la fenêtre
cargo tauri build          # binaire + paquets dans target/release
```

`build` produit le binaire `samflix-launcher` et, sous `bundle/`, un `.deb`, un
`.rpm` et une `.AppImage`. Les paquets déclarent `libwebkit2gtk-4.1` et
`libgtk-3` et posent le `.desktop` et les icônes ; l'AppImage les embarque et se
lance telle quelle.

Rien à poser dans l'environnement : sous NVIDIA, WebKitGTK rendait une fenêtre
blanche tant qu'on ne lui passait pas `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Le
programme le fait pour lui-même au premier appel de `run()`, et seulement si le
module noyau `nvidia` est chargé — voir `src-tauri/src/webkit.rs`. Poser la
variable à la main reste possible et l'emporte, dans les deux sens.

## Disposition

Deux dossiers, et rien d'autre à la racine que ce qui décrit le dépôt.

```
crates/          tout le Rust, un seul workspace
├── mc-auth/     Microsoft → Xbox → XSTS → Minecraft, et le trousseau
├── mc-dl/       téléchargement vérifié, et l'observation de sa progression
├── mc-java/     détection et installation du runtime
├── mc-instance/ Minecraft, NeoForge, la ligne de commande du jeu
├── mc-mods/     résolution et déploiement des mods
├── mc-pack/     l'orchestration : install, jeu, verrou — et la CLI
├── mc-log/      journalisation, censure, incidents
├── mc-essais/   un serveur HTTP local, pour les suites
└── mc-app/      l'application Tauri : tauri.conf.json, icônes, src/

web/             tout le front
├── src/app/     app.ts (l'écran), launcher.ts (le pont), format.ts
├── public/      les assets servis tels quels
└── dist/        la sortie de build (ignorée)

target/          la sortie de Cargo (ignorée)
docs/            ce fichier et ses voisins
```

`mc-app` est un membre du workspace comme les autres : un seul `cargo test`, un
seul verrou, une seule mesure de couverture. Ses chemins remontent d'un cran —
`frontendDist` vaut `../../web/dist/launcher/browser` — parce que Tauri résout
le répertoire de l'application relativement à `tauri.conf.json`.

## La cinématique

Onze phases, dessinées **en entier dès l'ouverture**, chacune portant son état :
faite, en cours, à venir. C'est ce qui distingue « on en est à la moitié » de
« il se passe quelque chose ».

| | |
|---|---|
| Compte Microsoft | session reprise du trousseau, ou ouverte par code d'appareil |
| Licence Minecraft | `Auth::owns_game` |
| Pack | manifeste et verrou |
| Version de NeoForge | `latest` résolu tout de suite, pour que le verrou porte un numéro |
| Fichiers du jeu | client, bibliothèques, assets |
| Java | détecté, ou Temurin installé |
| Chargeur NeoForge | l'installateur officiel, dans le Java ci-dessus |
| Mods | résolus, téléchargés, répartis client/serveur |
| Verrou | écrit en dernier : il décrit ce qui a réellement été fait |
| Prêt à jouer | le bouton s'allume |
| Jeu lancé | |

L'ordre des sept du milieu est celui de `mc_pack::install`, et il n'est pas
arbitraire : chaque étape dépend de la précédente. Voir [packs.md](packs.md).

**Un seul écart avec la ligne de commande, délibéré** : la licence est vérifiée
*avant* d'installer. `mc-pack` ne la regarde jamais, et faire attendre huit
cents mégaoctets à un joueur pour lui apprendre ensuite que son compte n'a pas
le jeu serait une faute.

**Installer et jouer restent deux gestes**, comme [lancement.md](lancement.md)
le pose : le bouton « Jouer » ne réinstalle rien.

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

## Ce que l'application fait, et ce qu'elle ne fait pas

Elle n'ajoute **aucune** logique de launcher. `mc-auth` authentifie,
`mc_pack::install` installe, `mc_pack::jeu` prépare et lance, `mc-log`
journalise. `mc-app` appelle, agrège, et raconte : c'est tout.

C'est délibéré, et cela s'est vérifié à l'usage. Le lancement vivait dans le
**binaire** `mc-pack`, sous `commandes/lancement/` : tant qu'il n'y avait qu'une
ligne de commande, cela ne coûtait rien. Dès que la fenêtre a voulu lancer le
jeu, elle n'a pas pu l'appeler — et a réassemblé la session, le Java du verrou
et la ligne de commande de son côté, en sautant au passage la vérification de
cohérence de l'instance. Deux assemblages pour la même chose, dont un seul
correct.

Le lancement est donc remonté dans la bibliothèque, et les deux appelants s'en
servent. Ce qui reste au binaire est ce qui lui appartient : l'affichage.

Même histoire pour le trousseau. Il a d'abord été écrit dans `mc-app`, et
`mc_auth::charger()` continuait de lire le fichier : la fenêtre enregistrait à
un endroit que la ligne de commande ne regardait pas. Il est dans `mc-auth`,
avec le reste du stockage de session, et les deux voient le même compte.

`mc-log` plutôt que `tauri-plugin-log`, d'ailleurs : le second écrirait les
jetons tels quels. Une fenêtre graphique avale sa sortie standard, donc le
fichier de journal est tout ce qu'un joueur pourra joindre à un rapport — c'est
exactement le moment où il ne faut pas qu'un jeton s'y trouve.

## Où va le jeton

L'état d'authentification porte un jeton de rafraîchissement : il rouvre le
compte sans mot de passe ni second facteur. Il va au **trousseau du système** —
Secret Service, Keychain, Credential Manager — sous le service `samflix-mc`.

Une machine sans trousseau retombe sur le fichier `0600` de `mc-auth`,
`~/.config/samflix-mc/session.json`, avec un `warn` dans le journal. La lecture
essaie les deux, dans cet ordre : une session ouverte par `mc-auth login` est
reprise ici sans rien redemander. Le détail, et pourquoi la déconnexion vide les
deux côtés, sont dans `src-tauri/src/coffre.rs`.

## Vérifier en production, pas seulement en `dev`

Les deux modes ne servent pas la même page. En `tauri dev`, c'est le serveur
Angular qui sert, sans CSP ; en production, c'est Tauri, avec le CSP de
`tauri.conf.json` et un nonce qu'il injecte lui-même. Or un nonce annule
`'unsafe-inline'` : tout ce qu'Angular injecterait à l'exécution — les styles
d'un composant, un gestionnaire `onload` posé par le « critical CSS » — est
alors rejeté, et la fenêtre s'affiche sans mise en forme. En `dev`, jamais.

D'où deux réglages qui n'ont l'air de rien : aucun `styleUrl` sur le composant
(tout est dans `src/styles.css`, chargé par `<link>`) et `inlineCritical: false`
dans `angular.json`. Le CSP peut alors rester strict — `style-src 'self'`, sans
`'unsafe-inline'`. La règle qui s'en déduit : une modification de l'interface se
regarde au moins une fois dans un `cargo tauri build`.

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

## L'écran

Une seule page, trois rangées, et **rien qui se déplace** d'un état à l'autre :
un héros, un contenu, et une barre d'action de hauteur fixe. C'est ce qui
distingue une application d'une succession de pages.

La barre d'action porte le compte à gauche et l'action à droite. Le bloc de
droite **permute** entre le bouton et la progression, à la même place : là où
il y aura « JOUER », il y a l'avancement. Rien ne saute quand l'installation
démarre.

Trois choix qui viennent de défauts constatés, et non d'une préférence :

- **La barre compte l'installation entière, pas le lot en cours.** Chaque étape
  annonce son propre lot, donc une barre par lot repasserait par zéro sept fois
  — et « 100 % » sept fois de suite fait douter qu'il se passe quelque chose.
- **Entre deux lots, la barre devient indéterminée.** La résolution des mods
  enchaîne jusqu'à six passes entrecoupées d'inspections de jars, et
  l'installateur NeoForge tourne une minute sans rien télécharger. Une barre
  pleine pendant ce temps ment ; une barre qui glisse dit « ça travaille ».
- **Une erreur prend tout le cadre et floute ce qu'il y a derrière.** Elle
  s'affichait en bas de page : dès qu'on avait fait défiler l'écran, on ne la
  voyait pas. Le flou la rend impossible à manquer, et son détail se copie.

La pastille du joueur est locale — ses initiales sur une teinte dérivée de son
UUID — plutôt qu'un avatar tiré d'un service comme `mc-heads` ou `crafatar` :
envoyer l'UUID d'un joueur à un tiers pour une image décorative n'en vaut pas
le prix. Si un vrai rendu de skin devient souhaitable, c'est un choix à faire
explicitement, avec le CSP qui va avec.

## Contrôles

```bash
pnpm --dir web test                                        # le front, vitest
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
