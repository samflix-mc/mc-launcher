# L'interface

[← README](../README.md)

L'application : une fenêtre **Tauri 2** dont le front est en **Angular 22**, sans
framework de style — du CSS et rien d'autre.

```bash
pnpm install           # une fois
cargo tauri dev        # démarre le serveur Angular puis la fenêtre
cargo tauri build      # binaire + paquets dans src-tauri/target/release
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

Celle que Tauri attend, à la racine du dépôt.

```
angular.json, package.json, tsconfig*.json   le front
src/                                          app.ts (l'écran), launcher.ts (le pont)
public/                                       favicon
src-tauri/src/                                lib.rs, commandes.rs, coffre.rs, webkit.rs
crates/                                       tout le reste — la logique du launcher
```

`src-tauri` est un membre du workspace Cargo comme les autres crates : un seul
`cargo test`, un seul verrou, une seule mesure de couverture.

## Ce que l'application fait, et ce qu'elle ne fait pas

Elle n'ajoute **aucune** logique de launcher. L'authentification est celle de
`mc-auth`, l'installation celle de `mc-pack`, le lancement celui de
`mc-instance`, la journalisation celle de `mc-log`. `src-tauri` est un pont :
des types sérialisables, des commandes, un coffre pour le jeton, et de quoi
montrer où en sont les crates pendant qu'elles travaillent.

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

## Contrôles

```bash
pnpm test                                                  # le front, vitest
cargo clippy --all-targets -- -D warnings && cargo test     # le reste
```
