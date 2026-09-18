# Journaux et remontée d'incidents

[← README](../README.md)

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

Les journaux vivent dans `~/.local/share/mc.samflix.launcher/logs/`.

## Ce qui ne sort pas

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

## Un plantage du jeu est un incident à part entière

Minecraft plante dans sa propre JVM : rien n'en arrive au launcher, sinon un
code de sortie. C'est pourtant le seul moment où la trace est disponible, et un
joueur ne pensera ni à la trouver ni à la joindre. Elle est donc lue dans les
journaux du jeu, censurée comme le reste, et envoyée comme une vraie exception
— le type Java sert de clé de regroupement, sans quoi tous les plantages
formeraient un unique incident « Minecraft s'est arrêté », inexploitable.

Les erreurs que le jeu rattrape en cours de partie remontent aussi : ce sont
souvent elles qui expliquent un comportement signalé bien plus tard.

## Environnements

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
