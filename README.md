<h1 align="center">mc-launcher</h1>

<p align="center">
  Le launcher du réseau <strong>samflix-mc</strong> : il installe le pack,<br>
  vérifie l'installation, et lance le jeu avec les mêmes mods que les serveurs.<br>
  <em>En développement — l'interface reste à faire.</em>
</p>

<p align="center">
  <img alt="Minecraft 1.21.1" src="https://img.shields.io/badge/Minecraft-1.21.1-5b8731?style=flat-square">
  <img alt="NeoForge 21.1" src="https://img.shields.io/badge/NeoForge-21.1-f16436?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-2024-b7410e?style=flat-square&logo=rust&logoColor=white">
</p>

<p align="center">
  <a href="https://github.com/samflix-mc/mc-launcher/actions/workflows/ci.yml"><img alt="Contrôles" src="https://github.com/samflix-mc/mc-launcher/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/samflix-mc/mc-launcher/actions/workflows/mutation.yml"><img alt="Mutation" src="https://github.com/samflix-mc/mc-launcher/actions/workflows/mutation.yml/badge.svg"></a>
  <a href="https://github.com/samflix-mc/mc-launcher/actions/workflows/audit.yml"><img alt="Vulnérabilités" src="https://github.com/samflix-mc/mc-launcher/actions/workflows/audit.yml/badge.svg"></a>
</p>

<p align="center">
  <a href="https://sonarcloud.io/summary/new_code?id=samflix-mc_mc-launcher"><img alt="Porte de qualité" src="https://sonarcloud.io/api/project_badges/measure?project=samflix-mc_mc-launcher&metric=alert_status"></a>
  <a href="https://sonarcloud.io/component_measures?id=samflix-mc_mc-launcher&metric=coverage"><img alt="Couverture" src="https://sonarcloud.io/api/project_badges/measure?project=samflix-mc_mc-launcher&metric=coverage"></a>
  <a href="https://sonarcloud.io/component_measures?id=samflix-mc_mc-launcher&metric=security_rating"><img alt="Sécurité" src="https://sonarcloud.io/api/project_badges/measure?project=samflix-mc_mc-launcher&metric=security_rating"></a>
</p>

---

Un serveur moddé impose que chaque joueur charge **exactement** les mêmes mods
que le serveur : les registres NeoForge sont négociés à la connexion, et une
version qui diffère d'un côté éjecte le joueur sans message exploitable. Ce
launcher supprime cette étape — il lit un pack publié, l'installe, et lance le
jeu avec ce que le pack déclare.

## Démarrer

```bash
mc-pack install                        # installe le pack publié
mc-pack verify [source] [--deep]       # l'installation est-elle conforme au verrou ?
mc-pack launch --pseudo Sam            # joue hors ligne
mc-auth  login                         # ou : ouvre une session Microsoft
mc-pack  launch                        # puis joue avec ce compte
mc-pack diagnostic                     # où sont les journaux, la télémétrie est-elle active ?
```

La liste des mods n'est pas décidée ici : elle est publiée par
[mc-content](https://github.com/samflix-mc/mc-content) et servie par
[mc-launcher-site](https://github.com/samflix-mc/mc-launcher-site). Le binaire
sait de quel environnement il vient, et c'est cela qui choisit le pack — voir
[packs.md](docs/packs.md).

## Où vivent les fichiers

```
~/.local/share/samflix-mc/
  shared/            versions/, libraries/, assets/ — forme d'un .minecraft
  instances/<nom>/minecraft/   mods, config, saves
  instances/<nom>/server/mods/ les mods du côté serveur
  runtime/temurin-21/
  cache/mods/
~/.config/samflix-mc/session.json      la session Microsoft, en 0600
```

Bibliothèques et assets pèsent près d'un gigaoctet et ne dépendent que de la
version du jeu : ils sont partagés entre instances. `shared/` a la forme d'un
`.minecraft` parce que l'installateur NeoForge l'exige.

## Les crates

| | |
|---|---|
| [`mc-pack`](crates/mc-pack) | manifeste, installation, vérification, lancement — **le binaire qu'on lance** |
| [`mc-mods`](crates/mc-mods) | résolution des mods : Modrinth, CurseForge, dépendances lues dans les jars |
| [`mc-instance`](crates/mc-instance) | Minecraft et NeoForge : installation, ligne de commande JVM, plantages |
| [`mc-auth`](crates/mc-auth) | authentification Microsoft, et profil hors-ligne |
| [`mc-java`](crates/mc-java) | détecte un Java 21, en installe un au besoin |
| [`mc-log`](crates/mc-log) | journaux console et fichier, incidents Sentry, censure des jetons |
| [`mc-dl`](crates/mc-dl) | téléchargements : reprise, empreintes, écriture atomique |
| [`mc-essais`](crates/mc-essais) | serveur HTTP d'essai, pour éprouver ce qui parle au réseau |

L'interface reste à faire ; tout le reste est écrit.

## Qualité

| | |
|---|---|
| Couverture | **94,4 %** des lignes |
| Score de mutation | **100 %** — aucun mutant ne survit |
| Mutants éprouvés | **96,4 %** (1012 sur 1050 ; les 38 écartés sont nommés dans le code) |

La couverture dit qu'une ligne a été *exécutée*, jamais que quelqu'un a regardé
ce qu'elle rendait. C'est la seconde mesure qui le dit : chaque pull request
passe au crible des mutants de son propre diff, avec un budget de **zéro
survivant**. Le détail est dans [qualite.md](docs/qualite.md).

## Documentation

| | |
|---|---|
| [packs.md](docs/packs.md) | d'où vient le pack, le manifeste, le verrou |
| [mods.md](docs/mods.md) | les trois sources, CurseForge sans clé, les dépendances cachées |
| [lancement.md](docs/lancement.md) | ce qui décide qu'un jeu démarre, le runtime Java |
| [authentification.md](docs/authentification.md) | Microsoft, mode hors-ligne, et ce que ce launcher présente |
| [qualite.md](docs/qualite.md) | les quatre workflows, couverture et mutation |

Le « pourquoi » de chaque décision est en tête du module concerné : c'est là
qu'il reste juste, et ces fichiers n'en sont que le résumé.

## Licence

MIT, à une exception près : `mc-auth` lie
[`minecraft-auth`](https://github.com/CCBlueX/minecraft-auth-rs) sous
LGPL-3.0-or-later. Ce que cela implique pour la distribution d'un binaire est
expliqué dans [authentification.md](docs/authentification.md).
