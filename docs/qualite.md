# Contrôles automatiques

[← README](../README.md)

Six workflows, qui se répondent sans se recouvrir.

Tous portent un `concurrency` qui annule l'exécution précédente sur la même
référence — **sauf `release.yml`**. Une publication interrompue laisse une
release à moitié peuplée, que rien ne distingue d'une complète.

## Contrôles — `ci.yml`

Trois jobs. `rust` et `front` en parallèle, puis `application` qui les rejoint.

| Job | Ce qu'il fait |
|---|---|
| `rust` | `cargo fmt --all --check`, `clippy --workspace`, `test --workspace`, et un contrôle de cohérence entre `members` et `default-members` |
| `front` | `format:check`, `invariants`, `ng test`, `ng build` |
| `application` | `tauri build --no-bundle`, avec `TAURI_APP_PATH` en absolu |

Trois points valent d'être connus :

- **`ubuntu-24.04`, et non `-arm`.** Le seul workflow qui garde chaque poussée
  n'exerçait jamais la branche `x64` de la détection Adoptium. L'ARM est
  devenue une jambe de la matrice de publication ;
- **le contrôle `members` = `default-members` ∪ `{mc-app}`.** Cargo refuse un
  paquet listé qui n'est pas membre, mais ne dit **rien** d'un membre absent de
  la liste : il sortirait alors du `cargo build` nu, de `clippy` sans
  `--workspace` et de la sélection par défaut de `cargo mutants`. C'est la seule
  omission dont le symptôme est l'absence de symptôme ;
- **`--no-bundle` sur les PR.** L'empaquetage complet n'apporte rien à la
  relecture d'un diff et coûte plusieurs minutes ; il a son entrée
  `workflow_dispatch`.

Ce que ce job **ne fait plus** : l'appel réel à Microsoft, parti dans
`externe.yml`.

## Services externes — `externe.yml`

Ce que la relecture du dépôt ne peut pas vérifier : les tiers dont le launcher
dépend et qui peuvent tomber, changer, ou refuser notre identité sans qu'une
ligne de code ait bougé. `mc-auth login` doit obtenir un code d'appareil — ce
qui vérifie la chaîne HTTP de bout en bout, sans secret et sans compte.

En cron quotidien, et non à chaque poussée : le contrôle coûtait **soixante
secondes pleines** à chaque fois, près d'un tiers du job. Ce n'est pas une
lenteur du réseau — `login` *interroge* Microsoft jusqu'à ce que quelqu'un
valide le code, personne ne le valide jamais ici, et le délai est donc atteint
en entier.

## Publication — `release.yml`

Quatre jobs, dont un garde. `coherence` compare le tag aux manifestes **avant
toute compilation** : `dpkg`, `rpm` et MSI comparent la version *interne*,
jamais le nom de fichier, et sans ce garde une release paraît livrée sans
l'être. Puis `ligne_de_commande` et `application` construisent en matrice, et
`publication` attache et réécrit les notes en français.

`permissions` n'est jamais déclaré globalement : chaque job pose la sienne —
`read` pour les trois premiers, `write` pour le dernier.

## Vulnérabilités — `audit.yml`

`cargo audit` sur les versions du verrou, à chaque changement et tous les
lundis, et `pnpm audit` sur l'arbre npm — le front est du code qu'un joueur
exécute, dans une WebView à qui `invoke` donne accès au système de fichiers et
au trousseau. Le rendez-vous hebdomadaire est le plus utile des deux : un avis
RustSec paraît sur une dépendance qu'on n'a pas touchée depuis des mois, et
sans lui il attendrait le prochain commit. Les vulnérabilités, le code
`unsound` et les versions retirées de crates.io arrêtent la CI ; les crates non
maintenues sont signalées sans bloquer.

**Une exception, nommée, avec sa date de péremption.** RUSTSEC-2024-0429 porte
sur `glib 0.18.5`, qui arrive par la chaîne GTK de Tauri : aucune version de
`glib` ne peut être choisie depuis ce dépôt, et le seul correctif est une
montée de gtk-rs côté Tauri. Ce qu'on accepte est borné — le launcher ne dépend
pas de `glib` et n'appelle nulle part le `VariantStrIter` en cause. L'ignorer
n'est pas desserrer la règle : c'est la garder, en nommant le seul cas où elle
n'apprend rien. Elle se retirera d'elle-même le jour venu, `cargo audit`
avertissant sur un `--ignore` sans correspondance.

## Qualité — `qualite.yml`

Couverture mesurée par `cargo llvm-cov`, puis analyse SonarQube Cloud. Deux
seuils, qui ne disent pas la même chose :

| | Portée | Valeur | Où |
|---|---|---|---|
| Cliquet | tout le dépôt | 90 % des lignes | `SEUIL_LIGNES` dans `qualite.yml` |
| Porte de qualité | code nouveau d'une PR | 80 % | Sonar, `sonar.qualitygate.wait` |

Le premier interdit de redescendre, le second exige 80 % de ce qu'on écrit
désormais.

### Ce qui sort de la couverture, et pourquoi c'est nommé

Un seuil qui porte sur du code dont personne ne peut écrire le test finit
contourné — on baisse le seuil — et il emporte alors les fichiers où il disait
vrai. Mieux vaut donc nommer ce qui ne peut pas être couvert.

Le critère est le même que celui qui écarte `mc-app` de la mutation : ces
fonctions demandent une application Tauri construite, donc un serveur
d'affichage. `run()` monte la fenêtre ; `chemins::poser` prend une `&App` ; les
commandes prennent un `AppHandle` et n'enveloppent que des appels dont chacun
est éprouvé dans le crate qui le porte.

```
crates/mc-app/src/lib.rs        crates/mc-app/src/commands.rs
crates/mc-app/src/main.rs       crates/mc-app/src/commandes/**
crates/mc-app/src/paths.rs
```

**Le reste de `mc-app` demeure dans le périmètre, et c'est le point.**
`csp.rs`, `acceptance.rs`, `brand.rs`, `navigation.rs`, `phase.rs`, `tracker.rs`,
`diagnostic.rs` et `cinematic.rs` ont tous une moitié pure, et elle est
testée — certaines à cent pour cent. Exclure le crate entier aurait été plus
simple, et aurait masqué cela.

Le front, lui, n'est pas exclu : `sonar.sources` porte sur `crates` **et**
`web/src`. La conséquence a été mesurée plutôt que supposée — la couverture du
front était à 20,5 %, et c'était toute la cause d'une porte rouge. La réponse a
été d'écrire les tests qui manquaient aux services de `noyau/`, pas d'élargir
l'exclusion. Le cliquet se remonte à la main, à mesure que le chiffre monte, et
reste quelques points sous le réel : il est là pour arrêter une chute, pas pour
rougir sur une variation d'un test.

```bash
cargo llvm-cov --workspace --locked --summary-only
```

L'analyse Sonar demande un projet créé sur [SonarCloud](https://sonarcloud.io)
et un secret `SONAR_TOKEN` dans les secrets Actions ; la marche à suivre exacte
est en tête de `sonar-project.properties`. Tant que le secret manque, le
workflow mesure la couverture, applique le cliquet et passe l'analyse — une CI
rouge faute de compte n'apprendrait rien à personne.

## Mutation — `mutation.yml`

La question que la couverture ne pose pas.

Une ligne couverte a été *exécutée* ; rien ne dit que quelqu'un a regardé ce
qu'elle rendait. Un test qui appelle une fonction sans rien vérifier la couvre
à 100 % et ne tombera pas le jour où elle rendra le contraire. `cargo-mutants`
change le code — une comparaison inversée, un retour remplacé par une valeur
par défaut, une branche supprimée — et regarde si la suite s'en aperçoit. Un
mutant qui **survit** désigne une ligne exécutée mais non vérifiée.

Les trois mesures se lisent ensemble, et aucune ne remplace l'autre :

| | Ce qu'elle mesure | Aujourd'hui |
|---|---|---|
| Couverture | lignes exécutées par la suite | **91,5 %** (`llvm-cov`, mesuré le 18/09/2026) |
| Mutants éprouvés | mutants soumis à la suite | **1246** |
| Score de mutation | mutants détectés, sur ceux qui compilent | **100 %** (0 survivant) |

Les deux premiers chiffres se rejouent en une commande chacun, et c'est
délibéré : un chiffre de documentation qu'on ne sait pas refaire est un chiffre
qu'on recopie.

```bash
cargo llvm-cov --workspace --locked --summary-only | tail -1
cargo mutants --list | wc -l
```

La couverture a **baissé** en fusionnant la fenêtre, et c'est attendu :
`mc-app` porte `run()`, qui demande un contexte graphique, et une poignée de
commandes qui enveloppent des appels. Le cliquet est à 90 %, ce qui laisse un
point et demi — il est là pour arrêter une chute, pas pour rougir sur la
variation d'un test.

Ce que le périmètre écarte, et pourquoi :

- **`mc-testkit`**, le harnais des suites — un serveur HTTP local, de faux
  runtimes, des archives fabriquées. Le muter reviendrait à demander qu'un test
  vérifie le harnais d'un autre test ;
- **`mc-app`**, par `default-members` et non par le fichier d'exclusions : il
  n'ajoute aucune logique, et chaque étape qu'il appelle est déjà mutée dans le
  crate qui la porte ;
- **21 fonctions** portant un `#[mutants::skip]`, chacune avec sa raison en
  regard — `grep -rn "mutants::skip" crates` les montre toutes ;
- **cinq motifs** dans `.cargo/mutants.toml`, pour des mutants **équivalents** :

  un « ou exclusif » entre deux drapeaux distincts, une garde qui n'est qu'un
  raccourci, un tour de boucle qui n'écrit rien. Aucun test ne peut les tuer,
  parce qu'il n'y a rien à distinguer.

Les `#[mutants::skip]` tiennent en trois familles, et rien d'autre : ce qui
parle à Microsoft à travers `minecraft-auth`, dont les adresses ne se
détournent pas vers un serveur d'essai ; ce qui télécharge chez Mojang,
NeoForge ou Adoptium par des adresses écrites en dur ; et ce qui n'écrit que
sur la sortie standard, que Rust ne sait pas relire depuis le processus qui
l'émet. Dans ce dernier cas, le texte est calculé par une fonction à part, elle
vérifiée — seule l'impression est écartée.

### Un mutant équivalent est souvent un détour

L'exclusion n'est pas le premier réflexe, et un exemple récent le montre. Dans
l'encodeur base64 de `mc-news`, deux mutants remplaçaient `|` par `^` dans

```rust
let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
```

Sur des octets rangés dans des champs de bits disjoints, les deux opérateurs
rendent le même nombre : aucun test ne pouvait les tuer. Écarter par un motif
aurait été correct — et aurait laissé le détour en place. La ligne est devenue

```rust
let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
```

Même calcul, meilleure lecture, et plus rien à muter.

Enfin, certains mutants **font boucler la suite sans fin** — remplacer `i += 1`
par `i *= 1` dans un analyseur, par exemple. Un dépassement de délai est une
façon d'être détecté, et ces mutants-là ne comptent pas parmi les survivants.
Ils disent en revanche quelque chose d'utile : c'est ainsi qu'on a découvert
qu'une boucle de l'analyseur de markdown ne garantissait pas sa progression sur
l'un de ses deux chemins. Un corps de billet vient d'un hôte et s'analyse dans
le fil d'interface : une boucle qui n'avance pas gèle la fenêtre entière, sans
message et sans autre issue que de tuer le processus.

### Comment le job tourne

Sur les pull requests, et sur leur diff seul : le dépôt entier demande une
douzaine de minutes réparties sur dix machines, une PR ordinaire quelques
minutes.

Le découpage est calculé à chaque exécution — `--list` n'exécute rien et rend
en quelques secondes le nombre exact de mutants du diff, d'où le nombre de
parts, à raison de cent par machine. Chaque part dit où elle en est pendant
qu'elle travaille (`… 37/100 éprouvés, 0 survivant, 4 min écoulées`), et le
total est jugé après coup : si une part n'aboutit pas, le job refuse de
conclure plutôt que de comparer un total incomplet au budget.

Le budget est à **zéro**. Un survivant qui paraît dans une PR n'est pas de la
dette héritée : c'est du code que cette PR vient d'écrire et que rien ne
vérifie.

```bash
cargo install cargo-mutants --locked
cargo mutants                      # tout le dépôt, une quinzaine de minutes
cargo mutants -f 'crates/mc-log/**'  # un crate
```
