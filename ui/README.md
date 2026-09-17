# ui — l'interface, en essai

[← README](../README.md)

Une maquette Tauri 2 + Angular 22, **à part du reste du dépôt** le temps de
vérifier que l'assemblage tient. Elle sait faire deux choses :

- ouvrir une session Microsoft et l'afficher ;
- un bouton « Lancer le jeu » qui dit qu'il n'est pas branché.

```bash
cd ui/src-tauri
cargo tauri dev        # démarre le serveur Angular puis la fenêtre
cargo tauri build      # binaire + paquets dans target/release
```

> Sous NVIDIA + Wayland, WebKitGTK ne démarre pas sans
> `WEBKIT_DISABLE_DMABUF_RENDERER=1`. La fenêtre se ferme aussitôt ouverte, avec
> une erreur DMA-BUF.

## Ce qui est branché, et ce qui ne l'est pas

| | état |
|---|---|
| Connexion Microsoft, par code d'appareil | réelle — c'est `mc-auth` |
| Reprise de la session au démarrage | réelle |
| Déconnexion | réelle |
| Lancement du jeu | **non** — le bouton rend un message |

Aucune logique de launcher n'est réécrite ici. `src-tauri` n'est qu'un pont :
des types sérialisables, quatre commandes, un coffre pour le jeton. Ce qui
viendra ensuite — pack, vérification, lancement — existe déjà dans les crates de
la racine et s'appellera de la même façon.

## Où va le jeton

L'état d'authentification porte un jeton de rafraîchissement : il rouvre le
compte sans mot de passe ni second facteur. Il va au **trousseau du système** —
Secret Service, Keychain, Credential Manager — sous le service `samflix-mc`.

Une machine sans trousseau retombe sur le fichier `0600` de `mc-auth`,
`~/.config/samflix-mc/session.json`, avec un `warn` dans le journal. La lecture
essaie les deux, dans cet ordre : une session ouverte par `mc-auth login` est
reprise ici sans rien redemander. Le détail, et pourquoi la déconnexion vide les
deux côtés, sont dans `src-tauri/src/coffre.rs`.

La fenêtre journalise par `mc-log`, pas par `tauri-plugin-log` : la censure des
jetons s'applique aussi à ce que le joueur joindra à un rapport.

## Pourquoi un workspace Cargo à part

`ui/src-tauri/Cargo.toml` porte un `[workspace]` vide. Sans lui, la chaîne
WebKit et les 600 dépendances de Tauri entreraient dans le `cargo test`, le
`cargo llvm-cov` et le `cargo mutants` du launcher, qui n'ont rien à y voir tant
que l'interface est un essai. Les crates de la racine sont tirées par chemin —
`mc-auth`, `mc-log` — et restent la source unique.

## Ce que ça devient ensuite

La cible est l'inverse de la disposition actuelle : **l'application Tauri à la
racine du dépôt**, `src-tauri/` à côté du front, les responsabilités réparties
en crates comme aujourd'hui. Ce dossier n'existe que pour répondre à « est-ce
que ça marche » avant de déplacer quoi que ce soit.

Le jour où la réponse est oui, la remontée tient en quatre gestes :

1. `ui/front/*` → à la racine, là où Tauri l'attend par défaut ;
2. `ui/src-tauri` → `src-tauri`, `[workspace]` vide retiré et ajouté aux
   `members` de la racine ;
3. dans `tauri.conf.json`, `beforeDevCommand`/`beforeBuildCommand` perdent leur
   `--dir front`, et `frontendDist` devient `../dist/<projet>/browser` ;
4. `crates/mc-auth` et `crates/mc-log` passent de `../../crates/…` à
   `../crates/…`.

Rien d'autre n'est à récrire : le pont ne connaît que les crates, et le front ne
connaît que le pont.

## Disposition

```
ui/
├── front/        Angular 22, sans framework de style — du CSS et rien d'autre
│   └── src/app/  app.ts (l'écran), launcher.ts (le seul qui parle à Rust)
└── src-tauri/
    └── src/      lib.rs (la fenêtre), commandes.rs (le pont), coffre.rs (le jeton)
```

## Contrôles

```bash
cd ui/front     && pnpm test              # 7 essais, vitest
cd ui/src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
```

Ce dossier n'est pas dans la CI : il n'est pas dans le workspace, donc ni le
cliquet de couverture ni les mutants ne le voient. C'est volontaire tant qu'il
est un essai, et à rattraper au moment de la remontée à la racine.
