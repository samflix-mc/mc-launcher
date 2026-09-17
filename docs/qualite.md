# Contrôles automatiques

[← README](../README.md)

Quatre workflows, qui se répondent sans se recouvrir.

## Contrôles — `ci.yml`

Format, `clippy -D warnings`, tests, compilation en release, et un appel réel à
Microsoft : `mc-auth login` doit obtenir un code d'appareil. La chaîne HTTP est
ainsi vérifiée de bout en bout sans qu'aucun secret n'entre dans la CI, et sans
compte.

## Vulnérabilités — `audit.yml`

`cargo audit` sur les versions du verrou, à chaque changement et tous les
lundis. Le rendez-vous hebdomadaire est le plus utile des deux : un avis
RustSec paraît sur une dépendance qu'on n'a pas touchée depuis des mois, et
sans lui il attendrait le prochain commit. Les vulnérabilités, le code
`unsound` et les versions retirées de crates.io arrêtent la CI ; les crates non
maintenues sont signalées sans bloquer.

## Qualité — `qualite.yml`

Couverture mesurée par `cargo llvm-cov`, puis analyse SonarQube Cloud. Deux
seuils, qui ne disent pas la même chose :

| | Portée | Valeur | Où |
|---|---|---|---|
| Cliquet | tout le dépôt | 90 % des lignes | `SEUIL_LIGNES` dans `qualite.yml` |
| Porte de qualité | code nouveau d'une PR | 80 % | Sonar, `sonar.qualitygate.wait` |

Le premier interdit de redescendre, le second exige 80 % de ce qu'on écrit
désormais. Le cliquet se remonte à la main, à mesure que le chiffre monte, et
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
| Couverture | lignes exécutées par la suite | **94,4 %** (Sonar ; 93,9 % pour `llvm-cov`, qui ne compte pas tout à fait les mêmes lignes) |
| Mutants éprouvés | mutants soumis à la suite, sur ceux que le code produit | **96,4 %** (1012 sur 1050) |
| Score de mutation | mutants détectés, sur ceux qui compilent | **100 %** (0 survivant) |

Le second chiffre est celui qui demande une explication. Sur les 1050 mutants
que produit le code de l'application — le harnais de test de `mc-essais` n'en
fait pas partie, muter un décor n'apprend rien —, 38 sont écartés d'avance :

- **31 par un `#[mutants::skip]`** posé sur la fonction, avec la raison en
  regard. Trois familles, et rien d'autre : ce qui parle à Microsoft à travers
  `minecraft-auth`, dont les adresses ne se détournent pas vers un serveur
  d'essai ; ce qui télécharge chez Mojang, NeoForge ou Adoptium par des
  adresses écrites en dur ; et ce qui n'écrit que sur la sortie standard, que
  Rust ne sait pas relire depuis le processus qui l'émet. Dans ce dernier cas,
  le texte est calculé par une fonction à part, elle vérifiée — seule
  l'impression est écartée ;
- **7 par `.cargo/mutants.toml`**, parce qu'ils ne changent rien : un « ou
  exclusif » entre deux drapeaux distincts, une garde qui n'est qu'un
  raccourci, un tour de boucle qui n'écrit rien. Aucun test ne peut les tuer,
  il n'y a rien à distinguer.

Des 1012 restants, 152 ne compilent pas — cargo-mutants les dit *non viables*,
et ils ne comptent nulle part — et 9 font boucler la suite sans fin, ce qui est
une façon d'être détectés. **Aucun ne survit.**

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
